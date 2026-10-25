//! The textures of the materials worked out on the processor (C4): the
//! same patterns as the 3D view's (`shaders.wgsl`, «Textures»), made into
//! square pictures that repeat without a seam. They are used by the
//! rendered pictures, by the `.glb` the project is sent to other programs
//! in, and by the 3D view on computers whose graphics card would be slow
//! with the patterns worked out at each pixel (a picture is only looked
//! up).
//!
//! Every pattern changes the material's colour in the same way: the
//! colour times a factor, plus a colour of its own (the mortar of a
//! brick, the chips of a terrazzo). A picture keeps both for each point,
//! so it serves any colour: `rgb` is what is added, `a` the factor (over
//! [`FACTOR`]).

use archisim_core::TextureKind;
use std::sync::{Arc, Mutex, OnceLock};

/// The factor is kept as a byte over this much.
pub const FACTOR: f32 = 1.6;

/// The pixels of each texture's picture along each side.
pub const SIZE: usize = 512;

/// A texture's picture: a square of `SIZE` × `SIZE`, repeating every
/// `period` pieces of the texture (a brick, a tile, a board).
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    pub kind: TextureKind,
    pub size: usize,
    pub period: f64,
    /// Rows from the bottom (`p.y` up), RGBA bytes: added colour and the
    /// factor over [`FACTOR`].
    pub rgba: Vec<u8>,
}

/// How many pieces of the texture its picture holds along each side:
/// enough for the pattern to repeat (boards are eight pieces long).
pub fn period(kind: TextureKind) -> f64 {
    match kind {
        TextureKind::Planks => 8.0,
        _ => 4.0,
    }
}

/// A colour as a factor of the material's plus a colour of its own.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Lin {
    a: f64,
    b: [f64; 3],
}

impl Lin {
    fn base(k: f64) -> Lin {
        Lin { a: k, b: [0.0; 3] }
    }
    fn fixed(c: [f64; 3]) -> Lin {
        Lin { a: 0.0, b: c }
    }
    fn mix(self, o: Lin, t: f64) -> Lin {
        Lin {
            a: self.a * (1.0 - t) + o.a * t,
            b: [0, 1, 2].map(|k| self.b[k] * (1.0 - t) + o.b[k] * t),
        }
    }
}

fn fract(x: f64) -> f64 {
    x - x.floor()
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn hash2(p: [f64; 2]) -> f64 {
    let q = [fract(p[0] * 123.34), fract(p[1] * 456.21)];
    let d = q[0] * (q[0] + 45.32) + q[1] * (q[1] + 45.32);
    let r = [q[0] + d, q[1] + d];
    fract(r[0] * r[1])
}

/// The lattice of a pattern that repeats every `period` pieces: a
/// frequency of whole cells over the period, and the cell coordinates
/// wrapped round it.
#[derive(Clone, Copy)]
struct Grid {
    f: [f64; 2],
    n: [f64; 2],
}

impl Grid {
    fn new(f: [f64; 2], period: f64) -> Grid {
        let n = [
            (f[0] * period).round().max(1.0),
            (f[1] * period).round().max(1.0),
        ];
        Grid {
            f: [n[0] / period, n[1] / period],
            n,
        }
    }
    fn wrap(&self, c: [f64; 2]) -> [f64; 2] {
        [c[0].rem_euclid(self.n[0]), c[1].rem_euclid(self.n[1])]
    }
    fn at(&self, p: [f64; 2]) -> [f64; 2] {
        [p[0] * self.f[0], p[1] * self.f[1]]
    }
    fn hash(&self, c: [f64; 2]) -> f64 {
        hash2(self.wrap(c))
    }
    /// Smooth noise at `p` (in pieces).
    fn noise(&self, p: [f64; 2]) -> f64 {
        self.noise_at(self.at(p))
    }
    /// Smooth noise at `x`, already in cells.
    fn noise_at(&self, x: [f64; 2]) -> f64 {
        let i = [x[0].floor(), x[1].floor()];
        let f = [fract(x[0]), fract(x[1])];
        let u = f.map(|v| v * v * (3.0 - 2.0 * v));
        let h = |dx: f64, dy: f64| self.hash([i[0] + dx, i[1] + dy]);
        let (a, b, c, d) = (h(0.0, 0.0), h(1.0, 0.0), h(0.0, 1.0), h(1.0, 1.0));
        let ab = a + (b - a) * u[0];
        let cd = c + (d - c) * u[0];
        ab + (cd - ab) * u[1]
    }
    /// Distance to the nearest and second nearest cell centres, and the
    /// nearest cell's id.
    fn cells(&self, p: [f64; 2]) -> [f64; 3] {
        let x = self.at(p);
        let i = [x[0].floor(), x[1].floor()];
        let (mut d1, mut d2, mut id) = (9.0f64, 9.0f64, 0.0);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let c = [i[0] + dx as f64, i[1] + dy as f64];
                let w = self.wrap(c);
                let o = [
                    hash2(w) * 0.8 + 0.1,
                    hash2([w[0] + 17.0, w[1] + 17.0]) * 0.8 + 0.1,
                ];
                let d = ((c[0] + o[0] - x[0]).powi(2) + (c[1] + o[1] - x[1]).powi(2)).sqrt();
                if d < d1 {
                    d2 = d1;
                    d1 = d;
                    id = hash2([w[0] + 3.0, w[1] + 3.0]);
                } else if d < d2 {
                    d2 = d;
                }
            }
        }
        [d1, d2, id]
    }
}

/// How far into a joint a coordinate is (0 in the joint, 1 away from it).
fn joint(t: f64, width: f64) -> f64 {
    let f = fract(t);
    smoothstep(width * 0.5, width, f.min(1.0 - f))
}

/// A frequency `k` (radians a piece) made whole over the period.
fn wave(k: f64, period: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    tau * (k * period / tau).round().max(1.0) / period
}

const MORTAR: [f64; 3] = [0.78, 0.76, 0.72];

/// The pattern at `p` (in pieces of the texture), as in the 3D view.
fn pattern(kind: TextureKind, p: [f64; 2]) -> Lin {
    let per = period(kind);
    let g = |fx: f64, fy: f64| Grid::new([fx, fy], per);
    let mortar = Lin::base(1.0).mix(Lin::fixed(MORTAR), 0.75);
    let [x, y] = p;
    match kind {
        // Brick: courses a quarter of its length high, every other one
        // moved half a brick.
        TextureKind::Brick => {
            let row = (y * 4.0).floor();
            let q = [x + 0.5 * row.rem_euclid(2.0), y * 4.0];
            let id = g(1.0, 4.0).hash([q[0].floor(), row]);
            let body = Lin::base((0.82 + 0.3 * id) * (0.93 + 0.1 * g(9.0, 9.0).noise(p)));
            mortar.mix(body, joint(q[0], 0.06) * joint(q[1], 0.2))
        }
        TextureKind::Stone => {
            let c = g(1.5, 1.5).cells(p);
            let edge = smoothstep(0.02, 0.09, c[1] - c[0]);
            let body = Lin::base((0.75 + 0.45 * c[2]) * (0.9 + 0.15 * g(6.0, 6.0).noise(p)));
            Lin::base(0.45).mix(body, edge)
        }
        TextureKind::Tile => {
            let id = g(1.0, 1.0).hash([x.floor(), y.floor()]);
            mortar.mix(Lin::base(0.95 + 0.08 * id), joint(x, 0.03) * joint(y, 0.03))
        }
        TextureKind::Wood => {
            let k = wave(40.0, per);
            let s = (y * k + g(1.5, 8.0).noise(p) * 6.0).sin();
            Lin::base(0.86 + 0.1 * s + 0.08 * g(0.5, 30.0).noise(p))
        }
        TextureKind::Planks => {
            let row = y.floor();
            let along = x / 8.0 + hash2([row.rem_euclid(per), 1.0]);
            // A board a row in each picture (it is eight pieces long).
            let id = hash2([row.rem_euclid(per), 2.0]);
            let s = (y * wave(25.0, per) + g(0.6, 5.0).noise(p) * 5.0).sin();
            let body = Lin::base((0.82 + 0.25 * id) * (0.93 + 0.06 * s));
            Lin::base(0.4).mix(body, joint(y, 0.06) * joint(along, 0.01))
        }
        TextureKind::Concrete => {
            let q = [x, y * 2.0];
            let m = joint(q[0], 0.008) * joint(q[1], 0.012);
            let f = [fract(q[0]), fract(q[1])];
            let d = |cx: f64, cy: f64| ((f[0] - cx).powi(2) + (f[1] - cy).powi(2)).sqrt();
            let hole = smoothstep(0.012, 0.022, d(0.15, 0.25).min(d(0.85, 0.25)));
            let body = 0.94 + 0.1 * g(12.0, 12.0).noise(p) + 0.04 * g(50.0, 50.0).noise(p);
            Lin::base(body * (0.82 + 0.18 * m) * (0.55 + 0.45 * hole))
        }
        TextureKind::Plaster => {
            Lin::base(0.95 + 0.07 * g(60.0, 60.0).noise(p) + 0.04 * g(7.0, 7.0).noise(p))
        }
        TextureKind::Grass => {
            let v = g(18.0, 40.0).noise(p) * 0.6 + g(4.0, 4.0).noise(p) * 0.4;
            Lin::base(0.7 + 0.55 * v)
        }
        TextureKind::Gravel => {
            let c = g(3.0, 3.0).cells(p);
            let body = Lin::base(0.7 + 0.5 * c[2]);
            Lin::base(0.5).mix(body, smoothstep(0.0, 0.25, c[1] - c[0]))
        }
        TextureKind::Water => {
            let inner = g(2.0, 2.0).noise(p) * 2.0;
            let w = g(3.0, 1.2);
            let at = w.at(p);
            let v = w.noise_at([at[0] + inner, at[1]]);
            Lin::base(0.88 + 0.25 * smoothstep(0.55, 0.8, v))
        }
        TextureKind::Metal => {
            let r = 0.5 + 0.5 * (x * std::f64::consts::TAU).sin();
            Lin::base(0.8 + 0.3 * r)
        }
        TextureKind::Linen => {
            let k = wave(150.0, per);
            let warp = 0.5 + 0.5 * (x * k).sin();
            let weft = 0.5 + 0.5 * (y * k).sin();
            let slub = g(4.0, 60.0).noise(p) * 0.6 + g(60.0, 4.0).noise(p) * 0.4;
            Lin::base(0.9 + 0.035 * (warp + weft) + 0.08 * slub + 0.04 * g(3.0, 3.0).noise(p))
        }
        TextureKind::Boucle => {
            let c = g(14.0, 14.0).cells(p);
            let loop_ = 1.0 - smoothstep(0.0, 0.45, c[0]);
            Lin::base(0.84 + 0.2 * loop_ + 0.06 * c[2])
        }
        TextureKind::Knit => {
            let q = [x * 10.0, y * 7.0];
            let col = fract(q[0]);
            let v = (col - 0.5).abs() * 2.0;
            let row = fract(q[1] + v * 0.5);
            let stitch = smoothstep(0.0, 0.35, row) * (1.0 - smoothstep(0.65, 1.0, row));
            let gap = smoothstep(0.0, 0.12, col.min(1.0 - col));
            Lin::base(0.78 + 0.22 * stitch * gap + 0.05 * g(2.0, 2.0).noise(p))
        }
        // Velvet: its pile; the sheen where the face turns away from the
        // eye is added where it is seen ([`velvet_sheen`]).
        TextureKind::Velvet => {
            let pile = g(12.0, 12.0).noise(p) * 0.5 + g(3.0, 3.0).noise(p) * 0.5;
            Lin::base(0.82 + 0.07 * pile)
        }
        TextureKind::Wicker => {
            let q = [x * 7.0, y * 7.0];
            let c = [q[0].floor(), q[1].floor()];
            let f = [fract(q[0]), fract(q[1])];
            let across = (c[0] + c[1]).rem_euclid(2.0);
            let t = if across > 0.5 { f[1] } else { f[0] };
            let strand = (t * std::f64::consts::PI).sin();
            let gap = smoothstep(0.0, 0.08, f[0].min(1.0 - f[0]).min(f[1]).min(1.0 - f[1]));
            Lin::base(0.55 + 0.5 * strand * gap + 0.05 * g(7.0, 7.0).hash(c))
        }
        TextureKind::Terrazzo => {
            let c = g(9.0, 9.0).cells(p);
            let chip = 1.0 - smoothstep(0.18, 0.24, c[0]);
            let tint = if c[2] > 0.66 {
                [0.75, 0.45, 0.35]
            } else if c[2] > 0.33 {
                [0.95, 0.93, 0.9]
            } else {
                [0.32, 0.3, 0.3]
            };
            let ground = Lin::base(0.96 + 0.05 * g(20.0, 20.0).noise(p));
            ground.mix(Lin::fixed(tint), chip * 0.85)
        }
        TextureKind::Marble => {
            let w = x * 1.5 + y * 0.5 + g(1.5, 1.5).noise(p) * 3.0 + g(5.0, 5.0).noise(p) * 0.8;
            let pi = std::f64::consts::PI;
            let vein = 1.0 - smoothstep(0.0, 0.06, (w * pi).sin().abs());
            // The shader's 9 a radian, made 3π so the picture repeats.
            let fine = 1.0
                - smoothstep(
                    0.0,
                    0.03,
                    ((w + g(9.0, 9.0).noise(p)) * 3.0 * pi).sin().abs(),
                );
            Lin::base((0.97 + 0.03 * g(4.0, 4.0).noise(p)) * (1.0 - 0.32 * vein - 0.12 * fine))
        }
        TextureKind::Soil => {
            let c = g(5.0, 5.0).cells(p);
            let clod = smoothstep(0.0, 0.3, c[1] - c[0]);
            let v = 0.78 + 0.25 * g(3.0, 3.0).noise(p) + 0.12 * g(17.0, 17.0).noise(p);
            let stone = (1.0 - smoothstep(0.1, 0.16, c[0])) * if c[2] >= 0.85 { 1.0 } else { 0.0 };
            Lin::base(v * (0.82 + 0.18 * clod)).mix(
                Lin {
                    a: 1.3,
                    b: [0.08, 0.08, 0.07],
                },
                stone,
            )
        }
        TextureKind::Leaves => {
            let c = g(2.5, 2.5).cells(p);
            let leaf = 1.0 - smoothstep(0.26, 0.33, c[0]);
            let tint = if c[2] > 0.66 {
                [0.66, 0.47, 0.18]
            } else if c[2] > 0.33 {
                [0.4, 0.2, 0.1]
            } else {
                [0.5, 0.31, 0.14]
            };
            Lin::base(0.72 + 0.25 * g(6.0, 6.0).noise(p)).mix(Lin::fixed(tint), leaf * 0.85)
        }
    }
}

/// The extra light of velvet where its face turns away from the eye
/// (`facing`: the cosine between the face and the line of sight).
pub fn velvet_sheen(facing: f64) -> f64 {
    let rim = 1.0 - facing.abs();
    0.45 * rim * rim
}

/// Makes the picture of a texture, `size` pixels a side, each the mean
/// of four points.
pub fn bake(kind: TextureKind, size: usize) -> Tile {
    let per = period(kind);
    let size = size.max(4);
    let mut rgba = vec![0u8; size * size * 4];
    let step = per / size as f64;
    for j in 0..size {
        for i in 0..size {
            let mut acc = Lin::base(0.0);
            for (dx, dy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                let l = pattern(kind, [(i as f64 + dx) * step, (j as f64 + dy) * step]);
                acc.a += l.a / 4.0;
                for k in 0..3 {
                    acc.b[k] += l.b[k] / 4.0;
                }
            }
            let o = (j * size + i) * 4;
            for k in 0..3 {
                rgba[o + k] = (acc.b[k].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            rgba[o + 3] = ((acc.a / FACTOR as f64).clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    Tile {
        kind,
        size,
        period: per,
        rgba,
    }
}

impl Tile {
    /// The factor and the added colour at `p` (in pieces), between the
    /// nearest pixels.
    pub fn at(&self, p: [f64; 2]) -> (f64, [f64; 3]) {
        let n = self.size;
        let fx = fract(p[0] / self.period) * n as f64 - 0.5;
        let fy = fract(p[1] / self.period) * n as f64 - 0.5;
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let px = |x: f64, y: f64| {
            let i = (x as i64).rem_euclid(n as i64) as usize;
            let j = (y as i64).rem_euclid(n as i64) as usize;
            let o = (j * n + i) * 4;
            [
                self.rgba[o] as f64 / 255.0,
                self.rgba[o + 1] as f64 / 255.0,
                self.rgba[o + 2] as f64 / 255.0,
                self.rgba[o + 3] as f64 / 255.0 * FACTOR as f64,
            ]
        };
        let (a, b, c, d) = (
            px(x0, y0),
            px(x0 + 1.0, y0),
            px(x0, y0 + 1.0),
            px(x0 + 1.0, y0 + 1.0),
        );
        let v: [f64; 4] = [0, 1, 2, 3].map(|k| {
            let ab = a[k] + (b[k] - a[k]) * tx;
            let cd = c[k] + (d[k] - c[k]) * tx;
            ab + (cd - ab) * ty
        });
        (v[3], [v[0], v[1], v[2]])
    }

    /// A material's colour (0–1) with this texture at `p`.
    pub fn apply(&self, base: [f64; 3], p: [f64; 2]) -> [f64; 3] {
        let (a, b) = self.at(p);
        [0, 1, 2].map(|k| (base[k] * a + b[k]).clamp(0.0, 1.5))
    }
}

/// The picture of a texture at the usual size, made once and kept.
pub fn tile(kind: TextureKind) -> Arc<Tile> {
    static TILES: OnceLock<Mutex<Vec<Arc<Tile>>>> = OnceLock::new();
    let tiles = TILES.get_or_init(|| Mutex::new(Vec::new()));
    if let Some(t) = tiles
        .lock()
        .ok()
        .and_then(|v| v.iter().find(|t| t.kind == kind).cloned())
    {
        return t;
    }
    let t = Arc::new(bake(kind, SIZE));
    if let Ok(mut v) = tiles.lock() {
        if !v.iter().any(|x| x.kind == kind) {
            v.push(t.clone());
        }
    }
    t
}

/// Where a point of a face is on its texture, as the 3D view takes it:
/// across the plan on faces looking up, along the face and up it on the
/// others; in pieces of `scale` metres.
pub fn texture_point(world: [f64; 3], normal: [f64; 3], scale: f64) -> [f64; 2] {
    let n = normal.map(f64::abs);
    let uv = if n[2] >= 0.7 {
        [world[0], world[1]]
    } else if n[0] > n[1] {
        [world[1], world[2]]
    } else {
        [world[0], world[2]]
    };
    let s = scale.max(0.01);
    [uv[0] / s, uv[1] / s]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pictures_repeat_without_a_seam() {
        for kind in TextureKind::ALL {
            let t = bake(kind, 64);
            // The left column against the right one, and the bottom row
            // against the top one: as close as two columns side by side
            // inside the picture.
            let col = |i: usize| -> Vec<i32> {
                (0..64)
                    .flat_map(|j| (0..4).map(move |k| (j, k)))
                    .map(|(j, k)| t.rgba[(j * 64 + i) * 4 + k] as i32)
                    .collect()
            };
            let diff = |a: &[i32], b: &[i32]| -> f64 {
                a.iter()
                    .zip(b)
                    .map(|(x, y)| (x - y).abs() as f64)
                    .sum::<f64>()
                    / a.len() as f64
            };
            let seam = diff(&col(63), &col(0));
            let inside = (1..63)
                .map(|i| diff(&col(i - 1), &col(i)))
                .fold(0.0, f64::max);
            assert!(
                seam <= inside + 1.0,
                "{kind:?}: seam {seam} inside {inside}"
            );
        }
    }

    /// With `TEXTURES_OUT=folder`, the pictures in a material's usual
    /// colour, to look at them.
    #[test]
    fn pictures_to_look_at() {
        let Some(dir) = std::env::var_os("TEXTURES_OUT") else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        for kind in TextureKind::ALL {
            let png = crate::glb::tile_png(kind, [190, 140, 100]);
            std::fs::write(dir.join(format!("{}.png", kind.code())), png).unwrap();
        }
    }

    #[test]
    fn a_brick_has_mortar_and_bricks() {
        let t = bake(TextureKind::Brick, 128);
        let red = [0.6, 0.25, 0.2];
        // In a joint: towards the mortar's grey; in a brick: red.
        let joint = t.apply(red, [0.3, 0.0]);
        let brick = t.apply(red, [0.3, 0.125]);
        assert!(joint[2] > brick[2] + 0.2, "{joint:?} {brick:?}");
        assert!(brick[0] > brick[2] * 2.0, "{brick:?}");
    }
}
