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
        TextureKind::Planks
        | TextureKind::Plaster
        | TextureKind::Water
        | TextureKind::Asphalt
        | TextureKind::Strata
        | TextureKind::EarthPath
        | TextureKind::Albero => 8.0,
        TextureKind::Grass | TextureKind::Sea => 16.0,
        TextureKind::SettsFan => 48.0,
        TextureKind::Clapboard => 20.0,
        TextureKind::Opus => 24.0,
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
    fn times(self, k: f64) -> Lin {
        Lin {
            a: self.a * k,
            b: self.b.map(|v| v * k),
        }
    }
    fn plus(self, c: [f64; 3]) -> Lin {
        Lin {
            a: self.a,
            b: [0, 1, 2].map(|k| self.b[k] + c[k]),
        }
    }
}

/// Distance from `t` to the nearest multiple of `size`.
fn edist(t: f64, size: f64) -> f64 {
    let f = fract(t / size);
    f.min(1.0 - f) * size
}

/// A 0–1 value in `n` steps (a painter's few tones).
fn tones(x: f64, n: f64) -> f64 {
    (x.min(0.999) * n).floor() / (n - 1.0).max(1.0)
}

/// 0 in a joint `half` wide each side of its line, 1 on the piece (the
/// pictures are supersampled: a hair of softening).
fn on_piece(e: f64, half: f64) -> f64 {
    smoothstep(half - 0.003, half + 0.003, e)
}

/// Odd rows.
fn odd(row: f64) -> f64 {
    if row.rem_euclid(2.0) >= 0.5 {
        1.0
    } else {
        0.0
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

/// The pattern at `p` (in pieces of the texture), as in the 3D view.
fn pattern(kind: TextureKind, p: [f64; 2]) -> Lin {
    let per = period(kind);
    let g = |fx: f64, fy: f64| Grid::new([fx, fy], per);
    let [x, y] = p;
    match kind {
        // Brick (stylized): four courses a brick's length, each brick one
        // of a few tones, bevelled, light joints.
        TextureKind::Brick => {
            let mortar = Lin::base(1.0).mix(Lin::fixed([0.82, 0.79, 0.73]), 0.78);
            let row = (y * 4.0).floor();
            let u = x + 0.5 * odd(row);
            let e = edist(u, 1.0).min(edist(y, 0.25));
            let id = g(1.0, 4.0).hash([u.floor(), row]);
            let mut k = 0.84 + 0.24 * tones(id, 4.0);
            if id > 0.88 {
                k *= 0.7;
            }
            if id < 0.08 {
                k *= 1.12;
            }
            k *= (0.95 + 0.07 * g(7.0, 7.0).noise(p)) * (0.97 + 0.08 * (fract(y * 4.0) - 0.3));
            mortar.mix(Lin::base(k), on_piece(e, 0.025))
        }
        // Stone (rubble): rounded stones, a light mortar between them.
        TextureKind::Stone => {
            let c = g(1.25, 1.9).cells(p);
            let e = (c[1] - c[0]) * 0.4;
            let mut body =
                Lin::base((0.82 + 0.28 * tones(c[2], 5.0)) * (0.94 + 0.08 * g(7.0, 7.0).noise(p)));
            if c[2] > 0.8 {
                body = body.mix(
                    Lin {
                        a: 0.86,
                        b: [0.06, 0.05, 0.02],
                    },
                    0.5,
                );
            }
            let j = Lin::base(1.0).mix(Lin::fixed([0.86, 0.84, 0.79]), 0.6);
            j.mix(body, on_piece(e, 0.025))
        }
        // Tiles and slabs: thin light joints, each its tone.
        TextureKind::Tile => {
            let e = edist(x, 1.0).min(edist(y, 1.0));
            let id = g(1.0, 1.0).hash([x.floor(), y.floor()]);
            let body = Lin::base(
                (0.93 + 0.1 * tones(id, 3.0))
                    * (0.96 + 0.06 * g(1.75, 1.75).noise(p) + 0.03 * g(30.0, 30.0).noise(p)),
            );
            let j = Lin::base(1.0).mix(Lin::fixed([0.93, 0.91, 0.87]), 0.7);
            j.mix(body, on_piece(e, 0.008))
        }
        TextureKind::Wood => {
            let k = wave(40.0, per);
            let s = (y * k + g(1.5, 8.0).noise(p) * 6.0).sin();
            Lin::base(0.86 + 0.1 * s + 0.08 * g(0.5, 30.0).noise(p))
        }
        // Boards (stylized): a few tones, painted grain, dark joints.
        TextureKind::Planks => {
            let row = y.floor();
            let along = x / 8.0 + hash2([row.rem_euclid(per), 1.0]);
            // A board a row in each picture (it is eight pieces long).
            let id = hash2([row.rem_euclid(per), 2.0]);
            let gr = g(0.17, 4.2).noise([x + id * 10.0 / 0.17, y]);
            let body =
                Lin::base((0.8 + 0.3 * tones(id, 4.0)) * (0.9 + 0.14 * smoothstep(0.3, 0.7, gr)));
            let e = edist(y, 1.0).min(edist(along, 1.0) * 8.0);
            Lin::base(0.45).mix(body, on_piece(e, 0.015))
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
        // Render (stylized): broad soft patches, a fine grain.
        TextureKind::Plaster => {
            let n = g(0.45, 0.45).noise(p) * 0.55
                + g(1.35, 1.35).noise(p) * 0.3
                + g(3.5, 3.5).noise(p) * 0.15;
            Lin::base(
                (0.95 + 0.08 * smoothstep(0.35, 0.65, n)) * (0.985 + 0.03 * g(45.0, 45.0).noise(p)),
            )
        }
        // Grass: soft patches, clumps of clover, strokes of blades.
        TextureKind::Grass => grass(p, per, 0.3),
        // Gravel: fine stones in three tones, domed.
        TextureKind::Gravel => {
            let c = g(3.0, 3.0).cells(p);
            let dome = 1.0 - smoothstep(0.0, 0.6, c[0]);
            let body = Lin::base(0.78)
                .mix(
                    Lin {
                        a: 0.9,
                        b: [0.05, 0.04, 0.03],
                    },
                    tones(c[2], 3.0) * 0.7,
                )
                .times(0.8 + 0.3 * dome);
            Lin::base(0.55).mix(body, smoothstep(0.0, 0.12, c[1] - c[0]))
        }
        // Water of a pool: the web of caustics (still, in a picture).
        TextureKind::Water => {
            let warp = [
                g(0.9, 0.9).noise(p) * 0.9 / 1.6,
                g(0.9, 0.9).noise([x + 5.0, y]) * 0.9 / 1.6,
            ];
            let c = g(1.6, 1.6).cells([x + warp[0], y + warp[1]]);
            let web = 1.0 - smoothstep(0.0, 0.09, c[1] - c[0]);
            Lin::base(1.0)
                .mix(Lin::fixed([0.72, 0.95, 1.0]), web * 0.85)
                .times(0.96 + 0.06 * g(2.0, 2.0).noise(p))
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
        // Ashlar: courses half a block high, each its block length (made
        // whole over the picture), chamfered.
        TextureKind::Ashlar => {
            let row = (y * 2.0).floor();
            let rows = g(1.0, 2.0);
            let n = (per / (0.8 + 0.4 * rows.hash([7.0, row]))).round().max(1.0);
            let bw = per / n;
            let u = x + rows.hash([3.0, row]) * bw;
            let e = edist(u, bw).min(edist(y, 0.5));
            let col = (u / bw).floor().rem_euclid(n);
            let id = hash2([col, row.rem_euclid(per * 2.0)]);
            let mut body = Lin::base(
                (0.88 + 0.16 * tones(id, 4.0))
                    * (0.93 + 0.1 * g(3.6, 3.6).noise(p) + 0.04 * g(24.0, 24.0).noise(p)),
            );
            if hash2([col + 5.0, row.rem_euclid(per * 2.0)]) > 0.75 {
                body = body.times(0.92).plus([0.02, 0.015, 0.0]);
            }
            let j = Lin::base(1.0).mix(Lin::fixed([0.9, 0.87, 0.8]), 0.6);
            j.mix(body, on_piece(e, 0.01))
        }
        // Clapboard: a shadow under each board's lip, butt joints every
        // twenty boards' widths.
        TextureKind::Clapboard => {
            let f = fract(y);
            let row = y.floor();
            let along = x / 20.0 + hash2([row.rem_euclid(per), 2.0]);
            let id = hash2([along.floor().rem_euclid(1.0), row.rem_euclid(per)]);
            let mut k = (0.95 + 0.07 * tones(id, 3.0)) * (0.97 + 0.05 * g(0.32, 11.2).noise(p));
            k *= (1.0 - 0.32 * smoothstep(0.82, 1.0, f)) * (0.92 + 0.08 * smoothstep(0.0, 0.25, f));
            let butt = on_piece(edist(along, 1.0) * 20.0, 0.02);
            Lin::base(k * (0.7 + 0.3 * butt))
        }
        // Shingles: staggered rows, each shingle its tone, a shadow under
        // each row.
        TextureKind::Shingles => {
            let f = fract(y);
            let row = y.floor();
            let n = (per / 1.2).round();
            let sw = per / n;
            let u = x / sw + hash2([row.rem_euclid(per), 9.0]);
            let id = hash2([u.floor().rem_euclid(n), row.rem_euclid(per)]);
            let mut c =
                Lin::base((0.8 + 0.32 * tones(id, 4.0)) * (0.95 + 0.08 * g(0.9, 5.4).noise(p)));
            if id > 0.85 {
                c = c.times(1.15).plus([0.02; 3]);
            }
            c = c.times(1.0 - 0.38 * smoothstep(0.78, 1.0, f));
            Lin::base(0.55).mix(c, on_piece(edist(u, 1.0) * sw, 0.035))
        }
        // Fish scales.
        TextureKind::Scales => {
            let n = (per / 1.5).round();
            let sw = per / n;
            let mut row = y.floor();
            let mut f = fract(y);
            let at = |row: f64| {
                let u = x / sw + 0.5 * odd(row);
                let xx = fract(u) * 2.0 - 1.0;
                (u, 0.45 * (1.0 - (1.0 - xx * xx).max(0.0).sqrt()))
            };
            let (mut u, mut bottom) = at(row);
            if f < bottom {
                row -= 1.0;
                f += 1.0;
                (u, bottom) = at(row);
            }
            let id = hash2([u.floor().rem_euclid(n), row.rem_euclid(per)]);
            let edge = smoothstep(0.0, 0.06, f - bottom);
            Lin::base(
                (0.82 + 0.28 * tones(id, 4.0))
                    * (0.55 + 0.45 * edge)
                    * (1.0 - 0.3 * smoothstep(0.85, 1.4, f)),
            )
        }
        // Barrel tiles: channels and caps, a lip at each course, some
        // weathered.
        TextureKind::Barrel => {
            // Half a channel along: the picture's edge falls inside one.
            let x = x + 0.5;
            let cl = per / (per / 1.8).round();
            let course = (y / cl).floor();
            let f = fract(y / cl);
            let r = (fract(x) * std::f64::consts::PI).sin();
            let id = hash2([x.floor().rem_euclid(per), course.rem_euclid(per)]);
            let worn = smoothstep(0.6, 0.95, g(0.18, 0.18).noise(p)) * 0.45;
            Lin::base(0.84 + 0.26 * tones(id, 4.0))
                .mix(
                    Lin {
                        a: 0.6,
                        b: [0.1, 0.07, 0.04],
                    },
                    worn,
                )
                .times((0.72 + 0.36 * r) * (1.0 - 0.35 * smoothstep(0.85, 1.0, f)))
        }
        // Standing seam: panels, light seams, patina in patches.
        TextureKind::Seam => {
            let e = edist(x, 1.0);
            let id = hash2([x.floor().rem_euclid(per), 1.0]);
            let pat = smoothstep(
                0.45,
                0.8,
                g(0.36, 0.16).noise(p) * 0.7 + g(1.8, 1.8).noise(p) * 0.3,
            );
            let mut c = Lin::base(0.95 + 0.08 * tones(id, 3.0)).mix(
                Lin {
                    a: 1.3,
                    b: [0.03, 0.05, 0.04],
                },
                pat * 0.6,
            );
            let rib = 1.0 - smoothstep(0.0, 0.03, e);
            c = c.mix(c.times(1.18), rib);
            c.times(1.0 - 0.18 * (1.0 - smoothstep(0.03, 0.09, e)) * (1.0 - rib))
        }
        // Setts: rounded stones in courses.
        TextureKind::Pavers => setts(p),
        // Asphalt: worn patches, light speckles.
        TextureKind::Asphalt => {
            let n = g(0.25, 0.25).noise(p) * 0.6 + g(1.3, 1.3).noise(p) * 0.4;
            let c = Lin::base(0.96 + 0.12 * tones(smoothstep(0.35, 0.8, n), 3.0));
            let k = g(30.0, 30.0).cells(p);
            let speck = (1.0 - smoothstep(0.05, 0.12, k[0])) * if k[2] >= 0.7 { 1.0 } else { 0.0 };
            c.mix(c.times(1.35), speck)
        }
        // Rock in strata: soft layers of different thickness and tone.
        TextureKind::Strata => {
            let mut z = y + g(0.14, 1.0).noise([x, 2.0]) * 1.6 + g(0.6, 1.0).noise([x, 7.0]) * 0.27;
            z += 0.8 * (z * 0.5).sin();
            let band = z.floor();
            let f = fract(z);
            let id = hash2([band.rem_euclid(per * 3.0), 5.0]);
            let mut k = 1.0 - 0.18 * tones(id, 4.0);
            k *= (1.0 + 0.14 * smoothstep(0.82, 0.97, f))
                * (1.0 - 0.22 * (1.0 - smoothstep(0.0, 0.18, f)));
            k *= 0.9 + 0.13 * g(1.3, 0.1).noise(p);
            if hash2([band.rem_euclid(per * 3.0), 9.0]) > 0.55 {
                k *= 1.0 - 0.35 * (1.0 - smoothstep(0.0, 0.07, f));
            }
            Lin::base(k)
        }
        // The sea (still, in a picture): broad tones and strokes.
        TextureKind::Sea => {
            let light = Lin {
                a: 1.3,
                b: [0.12, 0.22, 0.24],
            };
            let wv = g(0.1, 0.25).noise(p) * 0.7 + g(0.5, 0.5).noise(p) * 0.3;
            let mut c =
                Lin::base(1.0).mix(Lin::base(1.0).mix(light, 0.3), smoothstep(0.4, 0.7, wv));
            let shift = g(0.08, 0.08).noise(p) * 6.0;
            let strokes = |off: f64, freq: f64, width: f64, dashes: f64| {
                let lines = (freq * per).round().max(1.0);
                let yy = (y + off) * lines / per + g(0.05, 0.2).noise([x + shift, y]) * 1.5;
                let li = yy.floor();
                let f = fract(yy);
                let line =
                    smoothstep(0.0, width * 0.4, f) * (1.0 - smoothstep(width * 0.6, width, f));
                let dg = g(0.4, 1.0);
                let at = dg.at([x + shift, 0.0]);
                let dash = smoothstep(
                    dashes,
                    dashes + 0.12,
                    dg.noise_at([
                        at[0] + li.rem_euclid(lines) * 13.0,
                        li.rem_euclid(lines) * 1.7,
                    ]),
                );
                line * dash
            };
            let deep = strokes(11.0, 0.28, 0.3, 0.4);
            let mid = strokes(0.0, 0.36, 0.22, 0.48);
            let crest = strokes(37.0, 0.3, 0.12, 0.62);
            c = c.mix(Lin::base(1.0).mix(light, 0.55), deep * 0.55);
            c = c.mix(Lin::base(1.0).mix(light, 0.85), mid * 0.75);
            c.mix(Lin::fixed([0.93, 0.97, 1.0]), crest * 0.8)
        }
        // Setts laid diagonally.
        TextureKind::SettsDiagonal => setts([x + y, y - x]),
        // Setts in fans: circles of arcs sixteen setts wide, in rows six
        // apart, the lower ones over the upper ones.
        TextureKind::SettsFan => {
            let (fw, rad, fh) = (16.0, 8.0, 6.0);
            let r0 = ((y - rad) / fh).floor();
            let mut found = None;
            let mut below = 99.0f64;
            for k in 0..4 {
                let r = r0 + k as f64;
                let off = odd(r) * 0.5 * fw;
                let cx = ((x - off) / fw + 0.5).floor() * fw + off;
                let c = [cx, r * fh];
                let dist = ((x - c[0]).powi(2) + (y - c[1]).powi(2)).sqrt();
                if dist < rad {
                    found = Some((c, r));
                    break;
                }
                below = below.min(dist - rad);
            }
            let (c, row) = found.unwrap_or(([x, y], 0.0));
            let d = [x - c[0], y - c[1]];
            let rr = (d[0] * d[0] + d[1] * d[1]).sqrt();
            let band = rr.floor();
            let rw = row.rem_euclid(per / fh);
            let along = d[0].atan2(d[1]) * (band + 0.5) + hash2([band, rw]);
            let e = edist(rr, 1.0)
                .min(edist(along, 1.0) * rr.max(1.0) / (band + 0.5).max(0.5))
                .min(below);
            let id = hash2([along.floor(), band + rw * 31.0 + c[0].rem_euclid(per)]);
            let k = (0.8 + 0.28 * tones(id, 4.0)) * if id > 0.8 { 0.85 } else { 1.0 };
            Lin::base(0.6).mix(Lin::base(k), on_piece(e, 0.05))
        }
        // Cobbles: rounded stones packed close.
        TextureKind::Cobbles => {
            let c = g(1.0, 1.0).cells(p);
            let e = (c[1] - c[0]) * 0.5;
            let round = 1.0 - smoothstep(0.05, 0.6, c[0]);
            let mut body = Lin::base(0.78 + 0.32 * tones(c[2], 5.0));
            if c[2] > 0.82 {
                body = body.times(0.8).plus([0.07, 0.06, 0.04]);
            }
            Lin::base(0.42).mix(body.times(0.72 + 0.4 * round), on_piece(e, 0.056))
        }
        // Old cobbles: uneven, worn, earth and a muted moss in the joints.
        TextureKind::OldCobbles => {
            let warp = [
                g(0.31, 0.31).noise(p) * 0.35,
                g(0.31, 0.31).noise([x + 4.0, y]) * 0.35,
            ];
            let cg = g(1.0, 1.3);
            let c = cg.cells([x + warp[0], y + warp[1]]);
            let e = (c[1] - c[0]) * 0.5;
            let mut body = Lin::base(0.74 + 0.36 * tones(c[2], 5.0));
            if c[2] > 0.75 {
                body = body.times(0.85).plus([0.06, 0.05, 0.03]);
            }
            body = body.times(0.88 + 0.2 * (1.0 - smoothstep(0.0, 0.45, c[0])));
            let moss =
                smoothstep(0.58, 0.8, g(0.45, 0.45).noise(p)) * (0.6 + 0.4 * g(7.0, 7.0).noise(p));
            let moss_c = 0.8 + 0.3 * g(13.0, 13.0).noise(p);
            let joint_c = Lin {
                a: 0.42,
                b: [0.03, 0.025, 0.01],
            }
            .mix(
                Lin::fixed([0.19, 0.22, 0.1].map(|v| v * moss_c)),
                moss * 0.6,
            );
            let half = 0.0625 * (0.7 + 0.6 * g(0.7, 0.7).noise(p));
            joint_c.mix(body, on_piece(e, half))
        }
        // Opus romanum: a module of 8 × 6 units.
        TextureKind::Opus => {
            let m = [(x / 8.0).floor(), (y / 6.0).floor()];
            let l = [x - m[0] * 8.0, y - m[1] * 6.0];
            let r = opus_piece(l);
            let e = (l[0] - r[0])
                .min(l[1] - r[1])
                .min(r[0] + r[2] - l[0])
                .min(r[1] + r[3] - l[1]);
            let mw = [m[0].rem_euclid(per / 8.0), m[1].rem_euclid(per / 6.0)];
            let id = hash2([mw[0] * 7.0 + r[0] * 3.0 + r[1], mw[1] * 7.0 + r[2]]);
            let mut body = Lin::base(
                (0.86 + 0.2 * tones(id, 4.0))
                    * (0.95 + 0.07 * g(0.75, 0.75).noise(p) + 0.03 * g(6.0, 6.0).noise(p)),
            );
            if id > 0.7 {
                body = body.times(0.88).plus([0.03, 0.02, 0.0]);
            }
            Lin::base(1.0)
                .mix(Lin::fixed([0.94, 0.91, 0.84]), 0.7)
                .mix(body, on_piece(e, 0.027))
        }
        // Flagstone: irregular flat stones, wide light joints.
        TextureKind::Flagstone => {
            let warp = [
                g(0.7, 0.7).noise(p) * 0.25,
                g(0.7, 0.7).noise([x + 3.0, y]) * 0.25,
            ];
            let c = g(1.0, 1.0).cells([x + warp[0], y + warp[1]]);
            let e = (c[1] - c[0]) * 0.5;
            let mut body = Lin::base(0.82 + 0.26 * tones(c[2], 5.0));
            if fract(c[2] * 3.7) > 0.7 {
                body = body.times(0.85).plus([0.1, 0.05, 0.02]);
            }
            body = body.times(0.94 + 0.08 * g(1.5, 1.5).noise(p) + 0.04 * g(12.5, 12.5).noise(p));
            Lin::base(1.0)
                .mix(Lin::fixed([0.85, 0.82, 0.76]), 0.7)
                .mix(body, on_piece(e, 0.025))
        }
        // Flagstones in grass.
        TextureKind::FlagsInGrass => {
            let c = g(1.0, 1.0).cells(p);
            let e = (c[1] - c[0]) * 0.5;
            let stone = smoothstep(0.17, 0.19, e);
            let body =
                Lin::base((0.84 + 0.24 * tones(c[2], 4.0)) * (0.94 + 0.08 * g(2.1, 2.1).noise(p)));
            let gl = grass(p, per, 0.7);
            let green = [0.18, 0.42, 0.07];
            let gr = Lin::fixed([0, 1, 2].map(|k| green[k] * gl.a + gl.b[k]));
            gr.mix(body, stone)
        }
        // Marble slabs.
        TextureKind::MarbleSlabs => {
            let id = g(1.0, 1.0).hash([x.floor(), y.floor()]);
            let q = [x + id * 7.0 / 0.6, y + id * 7.0 / 0.6];
            let mg = |f: f64| g(0.6 * f, 0.6 * f).noise(q);
            let pi = std::f64::consts::PI;
            let wv = q[0] * 0.6 * 1.5 + q[1] * 0.6 * 0.5 + mg(1.5) * 3.0 + mg(5.0) * 0.8;
            let vein = 1.0 - smoothstep(0.0, 0.06, (wv * pi).sin().abs());
            let fine = 1.0 - smoothstep(0.0, 0.03, ((wv + mg(9.0)) * 3.0 * pi).sin().abs());
            let body =
                Lin::base((0.97 + 0.04 * tones(id, 3.0)) * (1.0 - 0.32 * vein - 0.12 * fine));
            let e = edist(x, 1.0).min(edist(y, 1.0));
            body.times(0.8 + 0.2 * on_piece(e, 0.003))
        }
        // Pebbles: oval stones of three colours, domed.
        TextureKind::Pebbles => {
            let c = g(1.0, 1.4).cells(p);
            let dome = 1.0 - smoothstep(0.0, 0.55, c[0]);
            let tint = if c[2] > 0.66 {
                Lin {
                    a: 0.4,
                    b: [0.5, 0.48, 0.44],
                }
            } else if c[2] < 0.33 {
                Lin {
                    a: 0.7,
                    b: [0.12, 0.1, 0.06],
                }
            } else {
                Lin::base(1.0)
            };
            let gap = smoothstep(0.03, 0.09, c[1] - c[0]);
            Lin::base(0.3).mix(tint.times(0.75 + 0.35 * dome), gap)
        }
        // Metro tiles: staggered oblongs, a glint along the top of each.
        TextureKind::MetroTiles => {
            let row = (y * 2.0).floor();
            let u = x + 0.5 * odd(row);
            let e = edist(u, 1.0).min(edist(y, 0.5));
            let id = g(1.0, 2.0).hash([u.floor(), row]);
            let fy = fract(y * 2.0);
            let glint = smoothstep(0.7, 0.86, fy) * (1.0 - smoothstep(0.86, 0.95, fy)) * 0.25;
            let body = Lin::base(0.96 + 0.06 * tones(id, 3.0)).mix(Lin::fixed([1.0; 3]), glint);
            Lin::base(1.0)
                .mix(Lin::fixed([0.85, 0.84, 0.81]), 0.7)
                .mix(body, on_piece(e, 0.0075))
        }
        // Patterned tiles: a star, a ring, petals and corners.
        TextureKind::PatternTiles => {
            let f = [fract(x) - 0.5, fract(y) - 0.5];
            let a = [f[0].abs(), f[1].abs()];
            let len = |u: f64, v: f64| (u * u + v * v).sqrt();
            let r = len(f[0], f[1]);
            let star = 1.0 - smoothstep(0.17, 0.18, (a[0] + a[1]).min(a[0].max(a[1]) * 1.25));
            let ring = smoothstep(0.27, 0.28, r) * (1.0 - smoothstep(0.31, 0.32, r));
            let corner = 1.0 - smoothstep(0.2, 0.21, len(a[0] - 0.5, a[1] - 0.5));
            let petals = (1.0 - smoothstep(0.06, 0.07, len(a[0] - 0.33, a[1])))
                + (1.0 - smoothstep(0.06, 0.07, len(a[0], a[1] - 0.33)));
            let body = Lin::fixed([0.85, 0.78, 0.6])
                .mix(Lin::base(1.0), (corner + ring + petals).clamp(0.0, 1.0))
                .mix(
                    Lin {
                        a: 0.45,
                        b: [0.35, 0.22, 0.02],
                    },
                    star,
                )
                .times(0.97 + 0.04 * g(8.0, 8.0).noise(p));
            let e = edist(x, 1.0).min(edist(y, 1.0));
            Lin::fixed([0.8, 0.77, 0.7]).mix(body, on_piece(e, 0.0075))
        }
        // An earth path: patches, little stones, a few cracks.
        TextureKind::EarthPath => {
            let n = g(0.6, 0.6).noise(p) * 0.6 + g(2.2, 2.2).noise(p) * 0.4;
            let mut c = Lin::base(0.88).mix(
                Lin {
                    a: 1.15,
                    b: [0.03, 0.025, 0.015],
                },
                tones(smoothstep(0.3, 0.75, n), 3.0) * 0.6,
            );
            let k = g(7.0, 7.0).cells(p);
            if k[2] >= 0.72 {
                c = c.mix(
                    Lin {
                        a: 0.7,
                        b: [0.3, 0.28, 0.24],
                    },
                    (1.0 - smoothstep(0.1, 0.17, k[0])) * 0.8,
                );
            }
            let k2 = g(1.3, 1.3).cells(p);
            if g(0.7, 0.7).noise(p) >= 0.5 {
                c = c.times(1.0 - 0.25 * (1.0 - smoothstep(0.0, 0.025, k2[1] - k2[0])));
            }
            c
        }
        // Albero: fine yellow sand.
        TextureKind::Albero => {
            let n = g(0.8, 0.8).noise(p) * 0.5
                + g(3.0, 3.0).noise(p) * 0.3
                + g(60.0, 60.0).noise(p) * 0.2;
            let mut k = 0.92 + 0.12 * tones(smoothstep(0.3, 0.75, n), 4.0);
            let c = g(25.0, 25.0).cells(p);
            if c[2] >= 0.8 {
                k *= 1.0 - 0.2 * (1.0 - smoothstep(0.05, 0.12, c[0]));
            }
            Lin::base(k)
        }
        // Furrows: a ploughed field, ridges broken into clods.
        TextureKind::Furrows => {
            let yy = y + g(0.12, 1.0).noise([x, 3.0]) * 0.5;
            let ridge = (fract(yy) * std::f64::consts::PI).sin();
            let c = g(7.0, 9.0).cells([x, yy]);
            let up = smoothstep(0.15, 0.7, ridge);
            let clod = (1.0 - smoothstep(0.1, 0.6, c[0])) * up;
            let mut body = Lin::base(0.5).mix(Lin::base(1.08), ridge);
            body = body.mix(Lin::base(0.4), (1.0 - smoothstep(0.0, 0.22, ridge)) * 0.55);
            body.times(
                (1.0 + (0.12 * tones(c[2], 3.0) - 0.04) * clod)
                    * (0.94 + 0.1 * g(0.6, 2.0).noise(p)),
            )
        }
        // A kitchen garden: lettuces and cabbages, leafy rosettes on ridges.
        TextureKind::KitchenGarden => {
            let f = fract(y);
            let row = y.floor();
            let rw = row.rem_euclid(per);
            let ridge = (f * std::f64::consts::PI).sin();
            let mut c = Lin::base(0.55).mix(Lin::base(1.02), ridge).times(
                (0.95 + 0.08 * g(2.4, 2.4).noise(p)) * (1.0 + 0.08 * (g(9.0, 9.0).noise(p) - 0.5)),
            );
            let sh = hash2([rw, 3.0]);
            let i = (x / 0.5 + sh).floor().rem_euclid(per / 0.5);
            let l = [fract(x / 0.5 + sh) - 0.5, (f - 0.5) * 2.0];
            let h = hash2([i, rw]);
            let cabbage = hash2([rw, 11.0]) > 0.5;
            let lobes = if cabbage { 6.0 } else { 9.0 };
            let size = 0.36 + 0.08 * h;
            let r = (l[0] * l[0] + l[1] * l[1]).sqrt();
            let ang = l[1].atan2(l[0]) + h * std::f64::consts::TAU;
            let frill = if cabbage { 0.0 } else { 0.025 };
            let rim =
                size * (0.92 + 0.07 * (ang * lobes).cos() + frill * (ang * lobes * 3.0).cos());
            let plant = 1.0 - smoothstep(rim - 0.03, rim, r);
            let heart_r = size * 0.55 * (0.92 + 0.08 * (ang * lobes + 1.6).cos());
            let heart = 1.0 - smoothstep(heart_r - 0.03, heart_r, r);
            let t = r / rim.max(0.001);
            let (light, dark) = if cabbage {
                ([0.36, 0.56, 0.4], [0.15, 0.33, 0.22])
            } else {
                ([0.42, 0.66, 0.16], [0.2, 0.42, 0.07])
            };
            let k = smoothstep(0.15, 1.0, t);
            let mut green: [f64; 3] = [0, 1, 2].map(|j| light[j] * (1.0 - k) + dark[j] * k);
            green = [0, 1, 2].map(|j| green[j] + (light[j] * 1.12 - green[j]) * heart * 0.7);
            let rib =
                (1.0 - smoothstep(0.0, 0.03, (ang * lobes * 0.5).sin().abs() * r)) * (1.0 - heart);
            green = [0, 1, 2].map(|j| {
                let to = light[j] * 1.2 + [0.06, 0.06, 0.04][j];
                green[j] + (to - green[j]) * rib * 0.5
            });
            let lit = 0.9 + 0.2 * (0.5 - l[1]);
            let d = ((l[0] - 0.08).powi(2) + (l[1] + 0.1).powi(2)).sqrt();
            let shadow = 1.0 - smoothstep(size * 0.9, size * 1.15, d);
            c = c.times(1.0 - 0.35 * shadow);
            c.mix(Lin::fixed(green.map(|v| v * lit)), plant)
        }
        // Cracked earth: dry plates, dark cracks.
        TextureKind::CrackedEarth => {
            let k = g(1.0, 1.0).cells(p);
            let e = (k[1] - k[0]) * 0.5;
            let curl = smoothstep(0.0, 0.5, k[0]);
            let body = Lin::base(
                (0.9 + 0.14 * tones(k[2], 3.0))
                    * (0.95 + 0.06 * g(2.8, 2.8).noise(p))
                    * (1.0 + 0.08 * curl),
            );
            Lin::base(0.45).mix(body, on_piece(e, 0.017))
        }
    }
}

/// Grass at `p` (in pieces of `scale` metres; the picture repeats every
/// `per` pieces): soft patches, clumps of clover, strokes of blades.
fn grass(p: [f64; 2], per: f64, scale: f64) -> Lin {
    let g = |fx: f64, fy: f64| Grid::new([fx, fy], per);
    let m = scale;
    let n = g(0.3 * m, 0.3 * m).noise(p) * 0.55
        + g(0.9 * m, 0.9 * m).noise(p) * 0.3
        + g(2.6 * m, 2.6 * m).noise(p) * 0.15;
    let mut c = Lin::base(0.86).mix(
        Lin {
            a: 0.75,
            b: [0.13, 0.19, 0.04],
        },
        tones(smoothstep(0.3, 0.7, n), 3.0),
    );
    let k = g(1.4 * m, 1.4 * m).cells(p);
    if k[2] >= 0.72 {
        c = c.times(1.0 - 0.12 * (1.0 - smoothstep(0.15, 0.4, k[0])));
    }
    // Strokes: a short dash in each little cell.
    let sg = g(9.0 * m, 9.0 * m);
    let q = sg.at(p);
    let i = [q[0].floor(), q[1].floor()];
    let (mut stroke, mut tone) = (0.0f64, 0.0);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let cc = [i[0] + dx as f64, i[1] + dy as f64];
            let w = sg.wrap(cc);
            let o = [hash2(w), hash2([w[0] + 17.0, w[1] + 17.0])];
            let a = (hash2([w[0] + 5.0, w[1] + 5.0]) - 0.5) * 1.4;
            let d = [a.sin(), a.cos()];
            let rel = [q[0] - cc[0] - o[0], q[1] - cc[1] - o[1]];
            let along = (rel[0] * d[0] + rel[1] * d[1]).clamp(-0.32, 0.32);
            let dist = ((rel[0] - d[0] * along).powi(2) + (rel[1] - d[1] * along).powi(2)).sqrt();
            let s1 = 1.0 - smoothstep(0.035, 0.07, dist);
            if s1 > stroke {
                stroke = s1;
                tone = hash2([w[0] + 9.0, w[1] + 9.0]);
            }
        }
    }
    let lit = if tone > 0.5 {
        c.times(1.22).plus([0.03, 0.04, 0.0])
    } else {
        c.times(0.78)
    };
    c.mix(lit, stroke * 0.75)
}

/// Setts in courses half a sett high (`q` in setts; the picture repeats
/// every four).
fn setts(q: [f64; 2]) -> Lin {
    let row = (q[1] * 2.0).floor();
    let u = q[0] + 0.5 * odd(row);
    let e = edist(u, 1.0).min(edist(q[1], 0.5));
    let id = hash2([u.floor().rem_euclid(4.0), row.rem_euclid(8.0)]);
    let k = (0.82 + 0.26 * tones(id, 4.0))
        * if id > 0.8 { 0.85 } else { 1.0 }
        * (0.95 + 0.07 * Grid::new([4.0, 4.0], 4.0).noise(q));
    Lin::base(0.6).mix(Lin::base(k), on_piece(e, 0.025))
}

/// The piece of the opus romanum's module (8 × 6 units) a point of it
/// falls in: x, y, width and height (as the 3D view's `opus_piece`).
fn opus_piece(l: [f64; 2]) -> [f64; 4] {
    let [x, y] = l;
    if x < 4.0 && y < 4.0 {
        [0.0, 0.0, 4.0, 4.0]
    } else if x >= 4.0 && y < 2.0 {
        [4.0, 0.0, 4.0, 2.0]
    } else if (4.0..6.0).contains(&x) && y < 4.0 {
        [4.0, 2.0, 2.0, 2.0]
    } else if x >= 6.0 {
        [6.0, 2.0, 2.0, 4.0]
    } else if x < 2.0 {
        [0.0, 4.0, 2.0, 2.0]
    } else if x < 4.0 {
        [2.0, 4.0, 2.0, 2.0]
    } else {
        [4.0, 4.0, 2.0, 2.0]
    }
}

/// The relief of a stylized texture at `p` (in pieces, up off the face):
/// in the pictures it is painted in as light from above (the 3D view
/// tilts the light with it).
fn relief(kind: TextureKind, p: [f64; 2]) -> f64 {
    let [x, y] = p;
    match kind {
        TextureKind::Brick => {
            let row = (y * 4.0).floor();
            let e = edist(x + 0.5 * odd(row), 1.0).min(edist(y, 0.25));
            smoothstep(0.025, 0.075, e).sqrt() * 0.048
        }
        TextureKind::Ashlar => smoothstep(0.01, 0.06, edist(y, 0.5)) * 0.035,
        TextureKind::Clapboard => {
            let f = fract(y);
            (1.0 - f) * 0.11 + 0.025 * smoothstep(0.0, 0.08, f)
        }
        TextureKind::Shingles => (1.0 - fract(y)) * 0.12,
        TextureKind::Barrel => {
            let r = (fract(x) * std::f64::consts::PI).sin();
            let cl = 4.0 / (4.0f64 / 1.8).round();
            r * 0.16 + (1.0 - fract(y / cl)) * 0.055
        }
        TextureKind::Pavers => {
            let row = (y * 2.0).floor();
            let e = edist(x + 0.5 * odd(row), 1.0).min(edist(y, 0.5));
            smoothstep(0.025, 0.085, e).sqrt() * 0.075
        }
        TextureKind::Furrows => {
            let g = Grid::new([0.12, 1.0], period(kind));
            let yy = y + g.noise([x, 3.0]) * 0.5;
            (fract(yy) * std::f64::consts::PI).sin() * 0.08
        }
        TextureKind::KitchenGarden => (fract(y) * std::f64::consts::PI).sin() * 0.05,
        _ => 0.0,
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
                let q = [(i as f64 + dx) * step, (j as f64 + dy) * step];
                let mut l = pattern(kind, q);
                // The relief, lit from above: lighter where it slopes
                // up towards the top, darker below a lip.
                let d = 0.01;
                let slope =
                    (relief(kind, [q[0], q[1] - d]) - relief(kind, [q[0], q[1] + d])) / (2.0 * d);
                if slope != 0.0 {
                    l = l.times(1.0 + (slope * 0.6).clamp(-0.3, 0.3));
                }
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
pub fn texture_point(world: [f64; 3], normal: [f64; 3], scale: f64, kind: TextureKind) -> [f64; 2] {
    let n = normal.map(f64::abs);
    let slopes = kind.on_slopes() || kind == TextureKind::Metal;
    let uv = if n[2] > 0.985 || (n[2] >= 0.7 && !slopes) {
        [world[0], world[1]]
    } else if kind == TextureKind::Strata {
        // Level layers that go on unbroken round corners and curves.
        [world[0] * 0.8 + world[1] * 0.6, world[2]]
    } else if slopes {
        // Roof coverings: along the eaves and up the slope (the 3D view's
        // frame on flat faces).
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
            .sqrt()
            .max(1e-9);
        let m = normal.map(|v| v / len);
        let h = (m[0] * m[0] + m[1] * m[1]).sqrt().max(1e-9);
        let t = [-m[1] / h, m[0] / h, 0.0];
        let b = [
            m[1] * t[2] - m[2] * t[1],
            m[2] * t[0] - m[0] * t[2],
            m[0] * t[1] - m[1] * t[0],
        ];
        let dot = |v: [f64; 3]| world[0] * v[0] + world[1] * v[1] + world[2] * v[2];
        [dot(t), dot(b)]
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
