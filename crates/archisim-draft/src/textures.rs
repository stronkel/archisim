//! Presentation textures: wood, metal, concrete and stone drawn piece by
//! piece at their real size, in thin lines with a hand-drawn touch. Every
//! board, stone or hole is a little different, and lines wobble and stop
//! short of their corners now and then, as a drawing by hand does.
//!
//! The pieces are laid out in the pattern's own axes, in metres (times
//! the hatch's scale), from the origin of the drawing: the same hatch
//! draws the same pieces wherever its boundary goes, and a piece keeps its
//! look when the boundary changes (its randomness comes from where it is,
//! not from the order it is drawn in). All of them are ArchiSim's own
//! designs.

use std::f64::consts::{PI, TAU};

use archisim_core::{HatchPattern, Point2};

use crate::hatch::{clip_segment, inside, noise, MAX_HATCH_LINES};
use crate::seg::dir;

/// Whether the pattern is one of these textures.
pub fn is_texture(p: HatchPattern) -> bool {
    use HatchPattern::*;
    matches!(
        p,
        WoodBoards
            | WoodVeins
            | WoodFinished
            | Pine
            | Oak
            | Maple
            | Birch
            | Ash
            | Walnut
            | Mahogany
            | MetalBrushed
            | MetalPerforated
            | DiamondPlate
            | Corrugated
            | MetalMesh
            | ConcreteFinished
            | ConcreteRough
            | ConcreteBare
            | ConcretePavers
            | BoardFormed
            | Pebbles
            | Limestone
            | StoneBlocks
            | Ashlar
            | Terrazzo
            | RockyPath
            | Setts
            | Rubble
    )
}

/// Most segments a texture draws before giving up (it is then drawn with
/// its plain stand-in lines).
const LIMIT: usize = MAX_HATCH_LINES * 4;

/// Draws the texture `p` over the polygons, clipped to them. `unit`: metres
/// of the drawing per metre of the pattern (the hatch's scale); `turn`:
/// the hatch's angle. `None` if it would be too dense to draw.
pub(crate) fn draw(
    polys: &[Vec<Point2>],
    p: HatchPattern,
    unit: f64,
    turn: f64,
    out: &mut Vec<(Point2, Point2)>,
) -> Option<()> {
    let u = dir(turn);
    let v = u.perp_left();
    let (mut x0, mut x1, mut y0, mut y1) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for q in polys.iter().flatten() {
        let (x, y) = (q.dot(u) / unit, q.dot(v) / unit);
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
        y1 = y1.max(y);
    }
    if !x0.is_finite() || unit <= 0.0 {
        return Some(());
    }
    let start = out.len();
    let mut c = Canvas {
        polys,
        u,
        v,
        unit,
        out,
        start,
        area: (x0, y0, x1, y1),
        full: false,
    };
    // A first guess of how many pieces the area would take: very large
    // areas go straight to the plain lines.
    let area = (x1 - x0) * (y1 - y0);
    if area * density(p) > LIMIT as f64 * 1.5 {
        return None;
    }
    use HatchPattern::*;
    match p {
        WoodBoards => boards(&mut c, &BOARDS),
        WoodVeins => boards(&mut c, &VEINS),
        WoodFinished => boards(&mut c, &FINISHED),
        Pine => boards(&mut c, &PINE),
        Oak => boards(&mut c, &OAK),
        Maple => boards(&mut c, &MAPLE),
        Birch => boards(&mut c, &BIRCH),
        Ash => boards(&mut c, &ASH),
        Walnut => boards(&mut c, &WALNUT),
        Mahogany => boards(&mut c, &MAHOGANY),
        MetalBrushed => brushed(&mut c),
        MetalPerforated => perforated(&mut c),
        DiamondPlate => diamond_plate(&mut c),
        Corrugated => corrugated(&mut c),
        MetalMesh => mesh(&mut c),
        ConcreteFinished => concrete_finished(&mut c),
        ConcreteRough => concrete_rough(&mut c),
        ConcreteBare => concrete_bare(&mut c),
        ConcretePavers => pavers(&mut c),
        BoardFormed => board_formed(&mut c),
        Pebbles => pebbles(&mut c),
        Limestone => limestone(&mut c),
        StoneBlocks => stone_blocks(&mut c),
        Ashlar => ashlar(&mut c),
        Terrazzo => terrazzo(&mut c),
        RockyPath => rocky_path(&mut c),
        Setts => setts(&mut c),
        Rubble => rubble(&mut c),
        _ => {}
    }
    if c.full {
        c.out.truncate(start);
        return None;
    }
    Some(())
}

/// About how many segments a square metre of the texture takes.
fn density(p: HatchPattern) -> f64 {
    use HatchPattern::*;
    match p {
        MetalPerforated => 2400.0,
        DiamondPlate => 1700.0,
        Terrazzo => 1700.0,
        Pebbles => 1500.0,
        ConcreteRough => 700.0,
        MetalMesh => 1100.0,
        _ => 300.0,
    }
}

/// Where a texture draws: the polygons, the pattern's axes and what has
/// been drawn.
struct Canvas<'a> {
    polys: &'a [Vec<Point2>],
    u: Point2,
    v: Point2,
    unit: f64,
    out: &'a mut Vec<(Point2, Point2)>,
    start: usize,
    /// The polygons' box in the pattern's axes.
    area: (f64, f64, f64, f64),
    /// Too many segments: stop.
    full: bool,
}

type P = (f64, f64);

/// A number in [0, 1) from a place: the same piece is always the same.
fn at_noise(p: P, k: u64) -> f64 {
    noise(
        (p.0 * 2000.0).round() as i64,
        (p.1 * 2000.0).round() as i64,
        k,
    )
}

/// A number in [a, b) from `n` in [0, 1).
fn lerp(a: f64, b: f64, n: f64) -> f64 {
    a + (b - a) * n
}

impl Canvas<'_> {
    fn world(&self, p: P) -> Point2 {
        self.u * (p.0 * self.unit) + self.v * (p.1 * self.unit)
    }

    /// Whether a box (pattern axes) touches the polygons' box.
    fn near(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> bool {
        let (a0, b0, a1, b1) = self.area;
        x1 >= a0 && x0 <= a1 && y1 >= b0 && y0 <= b1
    }

    fn raw(&mut self, a: P, b: P) {
        if self.full {
            return;
        }
        let (wa, wb) = (self.world(a), self.world(b));
        clip_segment(self.polys, wa, wb, self.out);
        if self.out.len() - self.start > LIMIT {
            self.full = true;
        }
    }

    /// A line drawn by hand: a gentle wobble across it, and its ends a
    /// little short of where they go now and then.
    fn line(&mut self, a: P, b: P) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        if len < 1e-9 {
            return;
        }
        let k = at_noise(a, 900) * 7.0 + at_noise(b, 901) * 13.0;
        let (ux, uy) = (dx / len, dy / len);
        let (nx, ny) = (-uy, ux);
        // Ends: sometimes stopping short (up to 2 % of the line).
        let t0 = if at_noise(a, 902) < 0.35 {
            0.03 * at_noise(a, 903)
        } else {
            0.0
        };
        let t1 = if at_noise(b, 904) < 0.35 {
            1.0 - 0.03 * at_noise(b, 905)
        } else {
            1.0
        };
        let amp = (0.007 * len).min(0.009);
        let pieces = ((len / 0.3).ceil() as usize).clamp(1, 10);
        if pieces == 1 || amp < 1e-4 {
            self.raw(
                (a.0 + dx * t0, a.1 + dy * t0),
                (a.0 + dx * t1, a.1 + dy * t1),
            );
            return;
        }
        let point = |t: f64| {
            // Zero at the ends, two waves with their own phases between.
            let w = (t * PI).sin()
                * amp
                * (0.7 * (t * TAU * 0.8 + k).sin() + 0.3 * (t * TAU * 2.3 + 2.0 * k).sin());
            (a.0 + dx * t + nx * w, a.1 + dy * t + ny * w)
        };
        let mut prev = point(t0);
        for i in 1..=pieces {
            let t = t0 + (t1 - t0) * i as f64 / pieces as f64;
            let q = point(t);
            self.raw(prev, q);
            prev = q;
        }
    }

    /// A polyline through `pts` by hand (closed: back to the first).
    fn path(&mut self, pts: &[P], closed: bool) {
        for w in pts.windows(2) {
            self.line(w[0], w[1]);
        }
        if closed && pts.len() > 2 {
            self.line(pts[pts.len() - 1], pts[0]);
        }
    }

    /// A polyline through `pts` as it is (curves already fine enough).
    fn curve(&mut self, pts: &[P], closed: bool) {
        for w in pts.windows(2) {
            self.raw(w[0], w[1]);
        }
        if closed && pts.len() > 2 {
            self.raw(pts[pts.len() - 1], pts[0]);
        }
    }

    fn dot(&mut self, p: P) {
        if self.full {
            return;
        }
        let q = self.world(p);
        if inside(self.polys, q) {
            self.out.push((q, q));
        }
    }

    /// A small closed blob around `c`: `n` points at radius `r` (`ratio`
    /// across, turned `angle`), each moved in or out up to `rough`.
    #[allow(clippy::too_many_arguments)]
    fn blob(&mut self, c: P, r: f64, ratio: f64, angle: f64, n: usize, rough: f64, salt: u64) {
        if !self.near(c.0 - r, c.1 - r, c.0 + r, c.1 + r) {
            return;
        }
        let (s, co) = angle.sin_cos();
        let pts: Vec<P> = (0..n)
            .map(|i| {
                let a = i as f64 / n as f64 * TAU;
                let k = 1.0 + rough * (2.0 * at_noise(c, salt + i as u64) - 1.0);
                let (x, y) = (r * k * a.cos(), r * ratio * k * a.sin());
                (c.0 + x * co - y * s, c.1 + x * s + y * co)
            })
            .collect();
        self.curve(&pts, true);
    }

    /// A circle of `n` sides.
    fn circle(&mut self, c: P, r: f64, n: usize) {
        if !self.near(c.0 - r, c.1 - r, c.0 + r, c.1 + r) {
            return;
        }
        let pts: Vec<P> = (0..n)
            .map(|i| {
                let a = i as f64 / n as f64 * TAU;
                (c.0 + r * a.cos(), c.1 + r * a.sin())
            })
            .collect();
        self.curve(&pts, true);
    }

    /// Dots scattered over a box, about `per_m2` per square metre, each in
    /// its own cell of a grid (so they never bunch up much).
    fn stipple(&mut self, per_m2: f64, salt: u64) {
        let (x0, y0, x1, y1) = self.area;
        let cell = 1.0 / per_m2.sqrt();
        for i in (x0 / cell).floor() as i64..=(x1 / cell).ceil() as i64 {
            for j in (y0 / cell).floor() as i64..=(y1 / cell).ceil() as i64 {
                let (a, b) = (noise(i, j, salt), noise(i, j, salt + 1));
                self.dot(((i as f64 + a) * cell, (j as f64 + b) * cell));
            }
            if self.full {
                return;
            }
        }
    }
}

/// Splits `[0, total)` into lengths between about `lo` and `hi`, as
/// break points from 0 to `total` (scaled so they end exactly there).
fn split(total: f64, lo: f64, hi: f64, seed: (i64, i64), salt: u64) -> Vec<f64> {
    let mut cuts = vec![0.0];
    let mut x = 0.0;
    let mut k = 0;
    while x < total - lo * 0.5 && k < 400 {
        x += lerp(lo, hi, noise(seed.0, seed.1 * 1000 + k, salt));
        cuts.push(x);
        k += 1;
    }
    let last = *cuts.last().unwrap_or(&total);
    if last > 0.0 {
        for c in &mut cuts {
            *c *= total / last;
        }
    }
    cuts
}

/// A piece of a course: from `x0` to `x1`, and a number of its own.
struct Piece {
    x0: f64,
    x1: f64,
    id: (i64, i64),
}

/// The pieces of course `row` over `[a, b)`: lengths between `lo` and
/// `hi`, laid in runs of `run` metres shifted course to course (so the
/// joints never line up).
fn pieces(row: i64, a: f64, b: f64, lo: f64, hi: f64, run: f64, salt: u64) -> Vec<Piece> {
    let shift = noise(row, 0, salt) * run;
    let mut out = Vec::new();
    for q in ((a - shift) / run).floor() as i64..=((b - shift) / run).ceil() as i64 {
        let base = q as f64 * run + shift;
        let cuts = split(run, lo, hi, (row, q), salt + 1);
        for (n, w) in cuts.windows(2).enumerate() {
            let (x0, x1) = (base + w[0], base + w[1]);
            if x1 >= a && x0 <= b {
                out.push(Piece {
                    x0,
                    x1,
                    id: (row, q * 1000 + n as i64),
                });
            }
        }
    }
    out
}

/// Courses of heights between `lo` and `hi` over `[a, b)`, in runs of
/// `run` metres: (bottom, top, number of the course).
fn courses(a: f64, b: f64, lo: f64, hi: f64, run: f64, salt: u64) -> Vec<(f64, f64, i64)> {
    let mut out = Vec::new();
    for q in (a / run).floor() as i64..=(b / run).ceil() as i64 {
        let cuts = split(run, lo, hi, (q, 7), salt);
        for (n, w) in cuts.windows(2).enumerate() {
            let (y0, y1) = (q as f64 * run + w[0], q as f64 * run + w[1]);
            if y1 >= a && y0 <= b {
                out.push((y0, y1, q * 1000 + n as i64));
            }
        }
    }
    out
}

// ─── Wood ────────────────────────────────────────────────────────────────

/// How a wood is drawn: boards along the pattern's x axis.
struct Wood {
    /// Board width, m.
    width: f64,
    /// Board lengths, m; `None`: continuous (no joints, no edges).
    length: Option<(f64, f64)>,
    /// Grain lines across a board.
    lines: usize,
    /// How far a grain line wanders across the board (share of the
    /// spacing between lines) and its wavelength, m.
    wander: f64,
    wave: f64,
    /// Share of lines that stop and start along the board.
    broken: f64,
    /// Share of boards with a cathedral figure, and its arches.
    cathedral: f64,
    arches: usize,
    /// Share of boards with a knot.
    knots: f64,
    /// Short flecks along the grain (rays, open pores) per m², and their
    /// length.
    flecks: f64,
    fleck: (f64, f64),
    /// Dots per m².
    dots: f64,
    /// Lines bunched in stripes (ribbon figure).
    ribbon: bool,
    salt: u64,
}

const BOARDS: Wood = Wood {
    width: 0.15,
    length: Some((1.0, 2.0)),
    lines: 3,
    wander: 0.35,
    wave: 1.6,
    broken: 0.3,
    cathedral: 0.35,
    arches: 3,
    knots: 0.08,
    flecks: 0.0,
    fleck: (0.0, 0.0),
    dots: 0.0,
    ribbon: false,
    salt: 100,
};
const VEINS: Wood = Wood {
    width: 0.22,
    length: None,
    lines: 5,
    wander: 0.5,
    wave: 2.2,
    broken: 0.15,
    cathedral: 0.8,
    arches: 5,
    knots: 0.04,
    flecks: 0.0,
    fleck: (0.0, 0.0),
    dots: 0.0,
    ribbon: false,
    salt: 200,
};
const FINISHED: Wood = Wood {
    width: 0.2,
    length: Some((2.0, 2.6)),
    lines: 2,
    wander: 0.2,
    wave: 3.0,
    broken: 0.8,
    cathedral: 0.1,
    arches: 2,
    knots: 0.0,
    flecks: 0.0,
    fleck: (0.0, 0.0),
    dots: 0.0,
    ribbon: false,
    salt: 300,
};
const PINE: Wood = Wood {
    width: 0.14,
    length: Some((1.5, 3.0)),
    lines: 4,
    wander: 0.45,
    wave: 1.4,
    broken: 0.2,
    cathedral: 0.6,
    arches: 4,
    knots: 0.45,
    flecks: 0.0,
    fleck: (0.0, 0.0),
    dots: 0.0,
    ribbon: false,
    salt: 400,
};
const OAK: Wood = Wood {
    width: 0.12,
    length: Some((0.6, 1.2)),
    lines: 4,
    wander: 0.15,
    wave: 2.5,
    broken: 0.35,
    cathedral: 0.2,
    arches: 3,
    knots: 0.03,
    flecks: 60.0,
    fleck: (0.008, 0.03),
    dots: 0.0,
    ribbon: false,
    salt: 500,
};
const MAPLE: Wood = Wood {
    width: 0.1,
    length: Some((0.5, 1.0)),
    lines: 2,
    wander: 0.12,
    wave: 3.5,
    broken: 0.6,
    cathedral: 0.0,
    arches: 0,
    knots: 0.0,
    flecks: 0.0,
    fleck: (0.0, 0.0),
    dots: 25.0,
    ribbon: false,
    salt: 600,
};
const BIRCH: Wood = Wood {
    width: 0.6,
    length: Some((1.2, 1.25)),
    lines: 16,
    wander: 0.12,
    wave: 1.1,
    broken: 0.25,
    cathedral: 0.0,
    arches: 0,
    knots: 0.15,
    flecks: 15.0,
    fleck: (0.004, 0.01),
    dots: 30.0,
    ribbon: false,
    salt: 700,
};
const ASH: Wood = Wood {
    width: 0.14,
    length: Some((1.0, 2.0)),
    lines: 4,
    wander: 0.3,
    wave: 2.0,
    broken: 0.2,
    cathedral: 0.75,
    arches: 4,
    knots: 0.02,
    flecks: 90.0,
    fleck: (0.004, 0.012),
    dots: 0.0,
    ribbon: false,
    salt: 800,
};
const WALNUT: Wood = Wood {
    width: 0.16,
    length: Some((1.2, 2.2)),
    lines: 6,
    wander: 0.4,
    wave: 0.7,
    broken: 0.25,
    cathedral: 0.3,
    arches: 4,
    knots: 0.12,
    flecks: 0.0,
    fleck: (0.0, 0.0),
    dots: 0.0,
    ribbon: false,
    salt: 900,
};
const MAHOGANY: Wood = Wood {
    width: 0.18,
    length: Some((2.0, 3.0)),
    lines: 7,
    wander: 0.08,
    wave: 4.0,
    broken: 0.15,
    cathedral: 0.0,
    arches: 0,
    knots: 0.0,
    flecks: 0.0,
    fleck: (0.0, 0.0),
    dots: 0.0,
    ribbon: true,
    salt: 1000,
};

fn boards(c: &mut Canvas, w: &Wood) {
    let (x0, y0, x1, y1) = c.area;
    let run = w.length.map_or(6.0, |(_, hi)| hi * 3.0);
    for row in (y0 / w.width).floor() as i64 - 1..=(y1 / w.width).ceil() as i64 {
        let (b0, b1) = (row as f64 * w.width, (row + 1) as f64 * w.width);
        let (lo, hi) = w.length.unwrap_or((run, run));
        for p in pieces(row, x0, x1, lo, hi, run, w.salt) {
            if w.length.is_some() {
                // The board's edge along it and its joint at the start.
                c.line((p.x0, b1), (p.x1, b1));
                c.line((p.x0, b0), (p.x0, b1));
            }
            grain(c, w, &p, b0, b1);
            if c.full {
                return;
            }
        }
    }
}

/// The grain of one board, from `p.x0` to `p.x1` between `b0` and `b1`.
fn grain(c: &mut Canvas, w: &Wood, p: &Piece, b0: f64, b1: f64) {
    let (id, salt) = (p.id, w.salt);
    let r = |k: u64| noise(id.0, id.1, salt + k);
    let width = b1 - b0;
    let len = p.x1 - p.x0;
    // A cathedral: arches pointing along the board around a centre.
    let cathedral = (w.arches > 0 && r(1) < w.cathedral).then(|| {
        let tip = p.x0 + len * lerp(0.35, 0.85, r(2));
        let centre = b0 + width * lerp(0.4, 0.6, r(3));
        let reach = len.min(1.4) * lerp(0.35, 0.6, r(4));
        (tip, centre, reach)
    });
    let half = width * 0.42;
    // The board's own wave, which all its lines follow (so they never
    // cross), each line adding a little of its own.
    let n = w.lines.max(1);
    let wave = w.wave * lerp(0.7, 1.3, r(5));
    let (phase, phase2) = (r(6) * TAU, r(7) * TAU);
    // Never more than lines can follow without crossing.
    let swell = (w.wander * width * 0.5).min(width * 0.22);
    let common = move |t: f64| {
        swell * ((t / wave * TAU + phase).sin() + 0.4 * (t / wave * TAU * 2.3 + phase2).sin())
    };
    for i in 0..n {
        let k = 10 + i as u64 * 7;
        let mut s = if w.ribbon {
            // Stripes: lines in two or three bunches across the board.
            let bunch = (i * 3 / n) as f64;
            (bunch + 0.5 + 0.35 * (r(k) - 0.5)) / 3.0
        } else {
            (i as f64 + 0.5 + 0.5 * (r(k) - 0.5)) / n as f64
        };
        s = b0 + width * s.clamp(0.08, 0.92);
        // A line's own share: small, so it stays clear of its neighbours.
        let amp = 0.25 * width / n as f64;
        let own = r(k + 1) * TAU;
        let own_wave = w.wave * lerp(0.4, 0.8, r(k + 2));
        // Lines near the edges follow the wave less (they are cut off).
        let follow = 1.0 - ((s - (b0 + b1) / 2.0).abs() / (width / 2.0)).powi(2);
        // Broken lines: one stretch of the board only.
        let (t0, t1) = if r(k + 3) < w.broken {
            let a = lerp(0.0, 0.5, r(k + 4));
            (
                p.x0 + len * a,
                p.x0 + len * (a + lerp(0.2, 0.5, r(k + 5))).min(1.0),
            )
        } else {
            (p.x0, p.x1)
        };
        // Lines through a cathedral stop where its arches are.
        let mut spans = vec![(t0, t1)];
        if let Some((tip, centre, reach)) = cathedral {
            if (s - centre).abs() < half {
                spans = vec![(t0, t1.min(tip - reach * 1.5)), (t0.max(tip + 0.03), t1)];
            }
        }
        for (a, b) in spans {
            if b - a < 0.03 || !c.near(a, b0, b, b1) {
                continue;
            }
            let at = |t: f64| {
                let y = s + follow * common(t) + amp * (t / own_wave * TAU + own).sin();
                (t, y.clamp(b0 + width * 0.04, b1 - width * 0.04))
            };
            let steps = (((b - a) / (wave.min(own_wave) / 10.0)).ceil() as usize).clamp(2, 240);
            let pts: Vec<P> = (0..=steps)
                .map(|q| at(a + (b - a) * q as f64 / steps as f64))
                .collect();
            c.curve(&pts, false);
        }
    }
    if let Some((tip, centre, reach)) = cathedral {
        // Nested arches: each a parabola with its tails running back
        // along the grain, wider and longer outwards.
        // Inner arches end sooner: the tips step back as the sides do.
        let step_back = reach * 0.5 / w.arches as f64;
        for a in 1..=w.arches {
            let f = a as f64 / w.arches as f64;
            let h = half * (0.15 + 0.85 * f);
            let end = tip - step_back * (w.arches - a) as f64;
            let l = reach * (0.25 + 0.35 * f);
            let tail = reach * lerp(0.5, 0.9, r(40 + a as u64));
            if !c.near(end - l - tail, centre - h - swell, end, centre + h + swell) {
                continue;
            }
            // A rounded arch (half an ellipse) with its tails.
            let mut pts: Vec<P> = Vec::new();
            pts.push((end - l - tail, centre - h));
            for q in 0..=16 {
                let ang = -PI / 2.0 + q as f64 / 16.0 * PI;
                pts.push((end - l + l * ang.cos(), centre + h * ang.sin()));
            }
            pts.push((end - l - tail, centre + h));
            let pts: Vec<P> = pts
                .into_iter()
                .map(|(x, y)| {
                    (
                        x,
                        (y + common(x)).clamp(b0 + width * 0.03, b1 - width * 0.03),
                    )
                })
                .collect();
            c.curve(&pts, false);
        }
    }
    // Knots: a few rings and a dot.
    if r(60) < w.knots {
        let k = (
            p.x0 + len * lerp(0.15, 0.85, r(61)),
            b0 + width * lerp(0.3, 0.7, r(62)),
        );
        let size = (width * lerp(0.08, 0.16, r(63))).min(0.05);
        for ring in 1..=3 {
            c.blob(k, size * ring as f64 / 2.0, 0.6, 0.0, 12, 0.12, 70 + ring);
        }
        c.dot(k);
    }
    // Flecks and dots, scattered over the board.
    let area = len * width;
    let flecks = (area * w.flecks).round() as u64;
    for f in 0..flecks {
        let q = (
            p.x0 + len * r(100 + 3 * f),
            b0 + width * lerp(0.05, 0.95, r(101 + 3 * f)),
        );
        let l = lerp(w.fleck.0, w.fleck.1, r(102 + 3 * f));
        c.raw(q, (q.0 + l, q.1));
    }
    let dots = (area * w.dots).round() as u64;
    for f in 0..dots {
        c.dot((
            p.x0 + len * r(400 + 2 * f),
            b0 + width * lerp(0.05, 0.95, r(401 + 2 * f)),
        ));
    }
}

// ─── Metal ───────────────────────────────────────────────────────────────

/// Brushed sheets 2 × 1 m with paired seams, and streaks along them.
fn brushed(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let (sw, sh) = (2.0, 1.0);
    for j in (y0 / sh).floor() as i64..=(y1 / sh).ceil() as i64 {
        let b = j as f64 * sh;
        let shift = if j.rem_euclid(2) == 1 { sw / 2.0 } else { 0.0 };
        for i in ((x0 - shift) / sw).floor() as i64..=((x1 - shift) / sw).ceil() as i64 {
            let a = i as f64 * sw + shift;
            if !c.near(a, b, a + sw, b + sh) {
                continue;
            }
            // Seams: two lines 6 mm apart.
            c.line((a, b), (a + sw, b));
            c.line((a, b + 0.006), (a + sw, b + 0.006));
            c.line((a, b), (a, b + sh));
            c.line((a + 0.006, b), (a + 0.006, b + sh));
            // Streaks of different lengths, along the sheet.
            for s in 0..140u64 {
                // Streaks gather in a few bands across the sheet.
                let band = (noise(i, j, 9 + s % 5) * 5.0).floor() / 5.0;
                let y = b + sh * (band + 0.2 * noise(i, j, 11 + 3 * s));
                let q = (a + sw * noise(i, j, 10 + 3 * s), y);
                let l = lerp(0.08, 0.6, noise(i, j, 12 + 3 * s));
                let end = (q.0 + l).min(a + sw);
                c.raw(q, (end, q.1));
            }
            if c.full {
                return;
            }
        }
    }
}

/// Round holes 25 mm in a staggered grid 60 mm apart.
fn perforated(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let s = 0.06;
    let h = s * 3f64.sqrt() / 2.0;
    for j in (y0 / h).floor() as i64 - 1..=(y1 / h).ceil() as i64 + 1 {
        let shift = if j.rem_euclid(2) == 1 { s / 2.0 } else { 0.0 };
        for i in ((x0 - shift) / s).floor() as i64 - 1..=((x1 - shift) / s).ceil() as i64 + 1 {
            let p = (i as f64 * s + shift, j as f64 * h);
            // A hair of difference hole to hole.
            let r = 0.0125 * (1.0 + 0.04 * (noise(i, j, 1) - 0.5));
            c.circle(p, r, 10);
        }
        if c.full {
            return;
        }
    }
}

/// Tread plate: raised lenses 40 mm long, crossing each other cell by
/// cell.
fn diamond_plate(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let g = 0.05;
    for j in (y0 / g).floor() as i64 - 1..=(y1 / g).ceil() as i64 + 1 {
        for i in (x0 / g).floor() as i64 - 1..=(x1 / g).ceil() as i64 + 1 {
            let centre = ((i as f64 + 0.5) * g, (j as f64 + 0.5) * g);
            let angle = if (i + j).rem_euclid(2) == 0 {
                PI / 4.0
            } else {
                -PI / 4.0
            };
            let (l, w) = (0.02, 0.0045);
            let (s, co) = angle.sin_cos();
            let at = |x: f64, y: f64| (centre.0 + x * co - y * s, centre.1 + x * s + y * co);
            if !c.near(centre.0 - l, centre.1 - l, centre.0 + l, centre.1 + l) {
                continue;
            }
            let pts = [
                at(-l, 0.0),
                at(-l * 0.5, -w),
                at(l * 0.5, -w),
                at(l, 0.0),
                at(l * 0.5, w),
                at(-l * 0.5, w),
            ];
            c.curve(&pts, true);
        }
        if c.full {
            return;
        }
    }
}

/// Trapezoidal profiled sheet seen face on: the edges of crests and
/// valleys every 180 mm, and light strokes down one flank.
fn corrugated(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let period = 0.18;
    // Edges of the profile within a period, and the shaded flank.
    let edges = [0.0, 0.04, 0.09, 0.13];
    let run = 1.5;
    for j in (y0 / period).floor() as i64 - 1..=(y1 / period).ceil() as i64 + 1 {
        let b = j as f64 * period;
        for i in (x0 / run).floor() as i64..=(x1 / run).ceil() as i64 {
            let a = i as f64 * run;
            if !c.near(a, b, a + run, b + period) {
                continue;
            }
            for e in edges {
                c.line((a, b + e), (a + run, b + e));
            }
            // The flank in shade: short strokes, a little uneven.
            let mut x = a;
            while x < a + run {
                let reach = lerp(0.3, 0.85, at_noise((x, b), 4));
                if at_noise((x, b), 5) < 0.8 {
                    c.raw((x, b + 0.088), (x + 0.006, b + 0.088 - 0.046 * reach));
                }
                x += lerp(0.02, 0.045, at_noise((x, b), 3));
            }
            if c.full {
                return;
            }
        }
    }
}

/// Expanded metal: rows of zigzags touching each other into diamonds
/// 100 × 40 mm.
fn mesh(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let (l, h) = (0.1, 0.04);
    let step = l / 2.0;
    for j in (y0 / h).floor() as i64 - 1..=(y1 / h).ceil() as i64 + 1 {
        let k0 = (x0 / step).floor() as i64 - 1;
        let k1 = (x1 / step).ceil() as i64 + 1;
        let y = |k: i64| j as f64 * h + if (k + j).rem_euclid(2) == 1 { h } else { 0.0 };
        let pts: Vec<P> = (k0..=k1).map(|k| (k as f64 * step, y(k))).collect();
        c.curve(&pts, false);
        // The strand's width: a second line where the strands meet.
        for k in k0..=k1 {
            if (k + j).rem_euclid(2) == 1 {
                let p = (k as f64 * step, y(k));
                c.raw((p.0 - 0.012, p.1 - 0.003), (p.0 + 0.012, p.1 - 0.003));
            }
        }
        if c.full {
            return;
        }
    }
}

// ─── Concrete ────────────────────────────────────────────────────────────

/// Fair-faced concrete: formwork panels 2,5 × 1,25 m, tie holes and a
/// few air bubbles.
fn concrete_finished(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let (pw, ph) = (2.5, 1.25);
    for j in (y0 / ph).floor() as i64..=(y1 / ph).ceil() as i64 {
        for i in (x0 / pw).floor() as i64..=(x1 / pw).ceil() as i64 {
            let (a, b) = (i as f64 * pw, j as f64 * ph);
            if !c.near(a, b, a + pw, b + ph) {
                continue;
            }
            c.line((a, b), (a + pw, b));
            c.line((a, b), (a, b + ph));
            for ti in 0..4 {
                for tj in 0..2 {
                    let p = (
                        a + 0.3125 + ti as f64 * 0.625,
                        b + 0.3125 + tj as f64 * 0.625,
                    );
                    c.circle(p, 0.012, 8);
                    c.circle(p, 0.006, 6);
                }
            }
        }
        if c.full {
            return;
        }
    }
    c.stipple(70.0, 30);
    bubbles(c, 10.0, 40);
    // Faint lift lines: where one pour met the next.
    let (x0, y0, x1, y1) = c.area;
    for j in (y0 / 0.6).floor() as i64..=(y1 / 0.6).ceil() as i64 {
        for i in (x0 / 1.7).floor() as i64..=(x1 / 1.7).ceil() as i64 {
            if noise(i, j, 45) < 0.45 {
                let y = j as f64 * 0.6 + 0.3 * noise(i, j, 46);
                let a = i as f64 * 1.7 + noise(i, j, 47);
                let l = lerp(0.3, 0.9, noise(i, j, 48));
                c.line((a, y), (a + l, y + 0.01 * (noise(i, j, 49) - 0.5)));
            }
        }
    }
}

/// Small round pores, about `per_m2` per square metre.
fn bubbles(c: &mut Canvas, per_m2: f64, salt: u64) {
    let (x0, y0, x1, y1) = c.area;
    let cell = 1.0 / per_m2.sqrt();
    for i in (x0 / cell).floor() as i64..=(x1 / cell).ceil() as i64 {
        for j in (y0 / cell).floor() as i64..=(y1 / cell).ceil() as i64 {
            let p = (
                (i as f64 + noise(i, j, salt)) * cell,
                (j as f64 + noise(i, j, salt + 1)) * cell,
            );
            c.circle(p, lerp(0.002, 0.005, noise(i, j, salt + 2)), 6);
        }
        if c.full {
            return;
        }
    }
}

/// Raw concrete: dense stippling and small angular aggregate.
fn concrete_rough(c: &mut Canvas) {
    c.stipple(380.0, 50);
    let (x0, y0, x1, y1) = c.area;
    let cell = 0.13;
    for i in (x0 / cell).floor() as i64..=(x1 / cell).ceil() as i64 {
        for j in (y0 / cell).floor() as i64..=(y1 / cell).ceil() as i64 {
            for n in 0..2u64 {
                let r = |k: u64| noise(i, j, 60 + 10 * n + k);
                let p = ((i as f64 + r(0)) * cell, (j as f64 + r(1)) * cell);
                let sides = 3 + (r(2) * 2.0) as usize;
                c.blob(
                    p,
                    lerp(0.004, 0.011, r(3)),
                    lerp(0.6, 1.0, r(4)),
                    r(5) * TAU,
                    sides,
                    0.3,
                    70 + 10 * n,
                );
            }
        }
        if c.full {
            return;
        }
    }
}

/// Bare concrete: light stippling, pores and a few hairline cracks.
fn concrete_bare(c: &mut Canvas) {
    c.stipple(220.0, 80);
    bubbles(c, 25.0, 90);
    let (x0, y0, x1, y1) = c.area;
    let cell = 1.1;
    for i in (x0 / cell).floor() as i64..=(x1 / cell).ceil() as i64 {
        for j in (y0 / cell).floor() as i64..=(y1 / cell).ceil() as i64 {
            if noise(i, j, 100) > 0.7 {
                continue;
            }
            // A crack: a wandering line with a branch.
            let mut p = (
                (i as f64 + noise(i, j, 101)) * cell,
                (j as f64 + noise(i, j, 102)) * cell,
            );
            let mut heading = noise(i, j, 103) * TAU;
            let mut pts = vec![p];
            let steps = 6 + (noise(i, j, 104) * 8.0) as u64;
            for k in 0..steps {
                heading += 0.9 * (noise(i, j, 110 + k) - 0.5);
                let l = lerp(0.03, 0.08, noise(i, j, 130 + k));
                p = (p.0 + l * heading.cos(), p.1 + l * heading.sin());
                pts.push(p);
            }
            c.curve(&pts, false);
            if pts.len() > 4 {
                let from = pts[pts.len() / 2];
                let h = heading + 0.8;
                c.raw(from, (from.0 + 0.06 * h.cos(), from.1 + 0.06 * h.sin()));
            }
        }
    }
}

/// Concrete pavers 200 × 100 mm in herringbone, with chamfered (paired)
/// joints.
fn pavers(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let a = 0.1;
    let gap = 0.003;
    // Bricks H(k + 2m, k − 2m) and V(k + 2m, k − 2m + 1), in units of `a`.
    let corners = [(x0, y0), (x0, y1), (x1, y0), (x1, y1)];
    let ks = corners.iter().map(|(x, y)| (x + y) / (2.0 * a));
    let ms = corners.iter().map(|(x, y)| (x - y) / (4.0 * a));
    let (k0, k1) = ks.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    });
    let (m0, m1) = ms.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    });
    if (k1 - k0) * (m1 - m0) > 4.0 * LIMIT as f64 {
        c.full = true;
        return;
    }
    for k in k0.floor() as i64 - 3..=k1.ceil() as i64 + 3 {
        for m in m0.floor() as i64 - 2..=m1.ceil() as i64 + 2 {
            let (ox, oy) = ((k + 2 * m) as f64 * a, (k - 2 * m) as f64 * a);
            for (bx, by, w, h) in [(ox, oy, 2.0 * a, a), (ox, oy + a, a, 2.0 * a)] {
                if !c.near(bx, by, bx + w, by + h) {
                    continue;
                }
                let r = [
                    (bx + gap, by + gap),
                    (bx + w - gap, by + gap),
                    (bx + w - gap, by + h - gap),
                    (bx + gap, by + h - gap),
                ];
                c.path(&r, true);
            }
        }
        if c.full {
            return;
        }
    }
}

/// Board-formed concrete: the print of boards 10–15 cm wide, their butt
/// joints, a little grain and the tie holes.
fn board_formed(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    for (b0, b1, row) in courses(y0, y1, 0.1, 0.15, 1.2, 1200) {
        for p in pieces(row, x0, x1, 1.8, 3.2, 8.0, 1210) {
            c.line((p.x0, b1), (p.x1, b1));
            c.line((p.x0, b0), (p.x0, b1));
            // One or two faint grain lines, broken.
            let r = |k: u64| noise(p.id.0, p.id.1, 1220 + k);
            for g in 0..2u64 {
                if r(g * 5) > 0.7 {
                    continue;
                }
                let s = b0 + (b1 - b0) * lerp(0.25, 0.75, r(g * 5 + 1));
                let a = p.x0 + (p.x1 - p.x0) * lerp(0.0, 0.5, r(g * 5 + 2));
                let l = (p.x1 - p.x0) * lerp(0.2, 0.5, r(g * 5 + 3));
                let amp = (b1 - b0) * 0.12;
                let phase = r(g * 5 + 4) * TAU;
                let pts: Vec<P> = (0..=20)
                    .map(|q| {
                        let t = a + l * q as f64 / 20.0;
                        (t, s + amp * (t / 1.3 * TAU + phase).sin())
                    })
                    .collect();
                if c.near(a, b0, a + l, b1) {
                    c.curve(&pts, false);
                }
            }
            if c.full {
                return;
            }
        }
    }
    // Tie holes on a 0,9 × 0,6 m grid.
    for i in (x0 / 0.9).floor() as i64..=(x1 / 0.9).ceil() as i64 {
        for j in (y0 / 0.6).floor() as i64..=(y1 / 0.6).ceil() as i64 {
            c.circle((i as f64 * 0.9 + 0.45, j as f64 * 0.6 + 0.3), 0.011, 8);
        }
    }
}

// ─── Stone ───────────────────────────────────────────────────────────────

/// Gravel: small rounded stones of different sizes, packed loosely.
fn pebbles(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let cell = 0.05;
    for i in (x0 / cell).floor() as i64 - 1..=(x1 / cell).ceil() as i64 + 1 {
        for j in (y0 / cell).floor() as i64 - 1..=(y1 / cell).ceil() as i64 + 1 {
            let r = |k: u64| noise(i, j, 1300 + k);
            // Grains of sand between the stones.
            c.dot(((i as f64 + r(1)) * cell, (j as f64 + r(2)) * cell));
            if r(0) > 0.9 {
                continue;
            }
            // Rows shifted, stones of all sizes, some angular.
            let shift = if j.rem_euclid(2) == 1 { 0.5 } else { 0.0 };
            let p = (
                (i as f64 + 0.5 + shift + 0.45 * (r(3) - 0.5)) * cell,
                (j as f64 + 0.5 + 0.45 * (r(4) - 0.5)) * cell,
            );
            let size = if r(8) < 0.2 {
                lerp(0.022, 0.03, r(5))
            } else {
                lerp(0.008, 0.02, r(5))
            };
            let sides = 5 + (r(9) * 4.0) as usize;
            c.blob(
                p,
                size,
                lerp(0.55, 0.95, r(6)),
                r(7) * PI,
                sides,
                0.22,
                1310,
            );
        }
        if c.full {
            return;
        }
    }
}

/// Limestone: large slabs in courses, with a few fossils and bedding
/// streaks.
fn limestone(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    for (b0, b1, row) in courses(y0, y1, 0.3, 0.45, 2.4, 1400) {
        for p in pieces(row, x0, x1, 0.55, 0.95, 3.0, 1410) {
            if !c.near(p.x0, b0, p.x1, b1) {
                continue;
            }
            c.line((p.x0, b1), (p.x1, b1));
            c.line((p.x0, b0), (p.x0, b1));
            let r = |k: u64| noise(p.id.0, p.id.1, 1420 + k);
            let (w, h) = (p.x1 - p.x0, b1 - b0);
            // A bedding streak, short and wavy.
            if r(0) < 0.5 {
                let s = b0 + h * lerp(0.2, 0.8, r(1));
                let a = p.x0 + w * lerp(0.05, 0.4, r(2));
                let l = w * lerp(0.3, 0.55, r(3));
                let pts: Vec<P> = (0..=10)
                    .map(|q| {
                        let t = a + l * q as f64 / 10.0;
                        (t, s + 0.006 * (t * 23.0 + r(4) * TAU).sin())
                    })
                    .collect();
                c.curve(&pts, false);
            }
            // Fossils: a small shell or two (an arc with its ribs).
            for f in 0..2u64 {
                if r(10 + f) > 0.35 {
                    continue;
                }
                let q = (
                    p.x0 + w * lerp(0.15, 0.85, r(12 + f)),
                    b0 + h * lerp(0.2, 0.8, r(14 + f)),
                );
                let size = lerp(0.008, 0.016, r(16 + f));
                let turn = r(18 + f) * TAU;
                let arc: Vec<P> = (0..=8)
                    .map(|a| {
                        let t = turn + a as f64 / 8.0 * PI;
                        (q.0 + size * t.cos(), q.1 + size * t.sin())
                    })
                    .collect();
                c.curve(&arc, false);
                for rib in [0.25, 0.5, 0.75] {
                    let t = turn + rib * PI;
                    c.raw(q, (q.0 + size * t.cos(), q.1 + size * t.sin()));
                }
            }
            // Fine pits.
            for d in 0..((w * h * 30.0) as u64) {
                c.dot((p.x0 + w * r(30 + 2 * d), b0 + h * r(31 + 2 * d)));
            }
            if c.full {
                return;
            }
        }
    }
}

/// Stone blocks: rough-faced blocks in courses of different heights, a
/// chipped corner here and there and a touch of shade under them.
fn stone_blocks(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let gap = 0.008;
    for (b0, b1, row) in courses(y0, y1, 0.22, 0.42, 2.4, 1500) {
        for p in pieces(row, x0, x1, 0.3, 0.85, 3.0, 1510) {
            if !c.near(p.x0, b0, p.x1, b1) {
                continue;
            }
            let r = |k: u64| noise(p.id.0, p.id.1, 1520 + k);
            let (a0, a1, c0, c1) = (p.x0 + gap, p.x1 - gap, b0 + gap, b1 - gap);
            let chip = (c1 - c0).min(a1 - a0) * lerp(0.1, 0.25, r(0));
            let mut pts = vec![(a0, c0)];
            // Bottom edge with a slight sag, right side, top, left.
            pts.push(((a0 + a1) / 2.0, c0 + 0.006 * (r(1) - 0.5)));
            if r(2) < 0.3 {
                pts.push((a1 - chip, c0));
                pts.push((a1, c0 + chip));
            } else {
                pts.push((a1, c0));
            }
            pts.push((a1 + 0.004 * (r(3) - 0.5), (c0 + c1) / 2.0));
            pts.push((a1, c1));
            if r(4) < 0.25 {
                pts.push((a0 + chip, c1));
                pts.push((a0, c1 - chip));
            } else {
                pts.push(((a0 + a1) / 2.0, c1 + 0.006 * (r(5) - 0.5)));
                pts.push((a0, c1));
            }
            c.path(&pts, true);
            // Shade: a few short strokes along the bottom right.
            let strokes = 2 + (r(6) * 3.0) as usize;
            for s in 0..strokes {
                let x = a1 - 0.02 - s as f64 * 0.018;
                c.raw((x, c0 + 0.006), (x + 0.012, c0 + 0.03));
            }
            if c.full {
                return;
            }
        }
    }
}

/// Ashlar: cut stone 600 × 300 mm in stretcher bond, each block with its
/// drafted margin, and tooling on some of them.
fn ashlar(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let (w, h, m) = (0.6, 0.3, 0.025);
    for j in (y0 / h).floor() as i64..=(y1 / h).ceil() as i64 {
        let b = j as f64 * h;
        let shift = if j.rem_euclid(2) == 1 { w / 2.0 } else { 0.0 };
        for i in ((x0 - shift) / w).floor() as i64..=((x1 - shift) / w).ceil() as i64 {
            let a = i as f64 * w + shift;
            if !c.near(a, b, a + w, b + h) {
                continue;
            }
            c.line((a, b), (a + w, b));
            c.line((a, b), (a, b + h));
            // The margin.
            let r = [
                (a + m, b + m),
                (a + w - m, b + m),
                (a + w - m, b + h - m),
                (a + m, b + h - m),
            ];
            c.path(&r, true);
            // Tooling: fine upright strokes, uneven in length.
            if noise(i, j, 1600) < 0.3 {
                let mut x = a + m + 0.02;
                while x < a + w - m - 0.015 {
                    let top = b + h - m - 0.012 - 0.06 * at_noise((x, b), 1601);
                    let bottom = b + m + 0.012 + 0.06 * at_noise((x, b), 1602);
                    c.raw((x, bottom), (x + 0.003, top));
                    x += lerp(0.025, 0.04, at_noise((x, b), 1603));
                }
            }
            if c.full {
                return;
            }
        }
    }
}

/// Terrazzo: chips of marble of all sizes, and fine grains between.
fn terrazzo(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let cell = 0.045;
    for i in (x0 / cell).floor() as i64 - 1..=(x1 / cell).ceil() as i64 + 1 {
        for j in (y0 / cell).floor() as i64 - 1..=(y1 / cell).ceil() as i64 + 1 {
            let r = |k: u64| noise(i, j, 1700 + k);
            if r(0) < 0.72 {
                let p = ((i as f64 + r(1)) * cell, (j as f64 + r(2)) * cell);
                // Mostly small chips, now and then a big one.
                let size = if r(3) < 0.12 {
                    lerp(0.012, 0.02, r(4))
                } else {
                    lerp(0.003, 0.009, r(4))
                };
                let sides = 3 + (r(5) * 3.0) as usize;
                c.blob(
                    p,
                    size,
                    lerp(0.55, 1.0, r(6)),
                    r(7) * TAU,
                    sides,
                    0.35,
                    1710,
                );
            }
            c.dot(((i as f64 + r(8)) * cell, (j as f64 + r(9)) * cell));
        }
        if c.full {
            return;
        }
    }
}

/// A rocky path: flat stones of different sizes laid close, rounded and
/// irregular, small stones and grit filling the gaps.
fn rocky_path(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let cell = 0.3;
    for i in (x0 / cell).floor() as i64 - 1..=(x1 / cell).ceil() as i64 + 1 {
        for j in (y0 / cell).floor() as i64 - 1..=(y1 / cell).ceil() as i64 + 1 {
            let r = |k: u64| noise(i, j, 1800 + k);
            let shift = if j.rem_euclid(2) == 1 { 0.5 } else { 0.0 };
            let p = (
                (i as f64 + 0.5 + shift + 0.3 * (r(0) - 0.5)) * cell,
                (j as f64 + 0.5 + 0.3 * (r(1) - 0.5)) * cell,
            );
            if !c.near(p.0 - cell, p.1 - cell, p.0 + cell, p.1 + cell) {
                continue;
            }
            // Big stones and smaller ones.
            let size = cell
                * if r(2) < 0.25 {
                    lerp(0.34, 0.4, r(8))
                } else {
                    lerp(0.44, 0.52, r(8))
                };
            // A smooth, irregular outline: harmonics of the radius.
            let (h2, h3, h4) = (r(3) * TAU, r(4) * TAU, r(9) * TAU);
            let ratio = lerp(0.72, 1.0, r(5));
            let turn = r(11) * PI;
            let (ts, tc) = turn.sin_cos();
            let pts: Vec<P> = (0..22)
                .map(|q| {
                    let a = q as f64 / 22.0 * TAU;
                    let k = 1.0
                        + 0.1 * (2.0 * a + h2).sin()
                        + 0.07 * (3.0 * a + h3).sin()
                        + 0.04 * (5.0 * a + h4).sin();
                    let (x, y) = (size * k * a.cos(), size * ratio * k * a.sin());
                    (p.0 + x * tc - y * ts, p.1 + x * ts + y * tc)
                })
                .collect();
            c.path(&pts, true);
            // A crack across some stones.
            if r(6) < 0.15 {
                let a = r(7) * PI;
                let d = (size * 0.6 * a.cos(), size * 0.6 * a.sin());
                c.line((p.0 - d.0, p.1 - d.1), (p.0 + d.0 * 0.4, p.1 + d.1 * 0.4));
            }
            // A small stone where three stones leave a gap.
            let g = ((i as f64 + 1.0) * cell, (j as f64 + 0.98) * cell);
            if r(12) < 0.6 {
                c.blob(
                    g,
                    cell * lerp(0.05, 0.09, r(13)),
                    0.7,
                    r(14) * PI,
                    7,
                    0.2,
                    1830,
                );
            }
            // Grit around it.
            for q in 0..6u64 {
                let a = r(20 + q) * TAU;
                let rr = size * lerp(1.08, 1.3, r(30 + q));
                c.dot((p.0 + rr * a.cos(), p.1 + rr * ratio * a.sin()));
            }
        }
        if c.full {
            return;
        }
    }
}

/// Setts in segmental arcs: courses 10 cm deep following arcs 1,4 m wide,
/// each sett about 10 cm, its joints square to the course and staggered
/// course to course.
fn setts(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let (period, depth, h) = (1.4, 0.2, 0.1);
    let sag = |x: f64| {
        let u = (x / period).rem_euclid(1.0) * 2.0 - 1.0;
        depth * (1.0 - u * u)
    };
    let slope = |x: f64| {
        let u = (x / period).rem_euclid(1.0) * 2.0 - 1.0;
        -4.0 * depth / period * u
    };
    let step = 0.04;
    for j in ((y0 - depth) / h).floor() as i64 - 1..=(y1 / h).ceil() as i64 + 1 {
        let base = j as f64 * h;
        if !c.near(x0, base, x1, base + depth + h) {
            continue;
        }
        // The course's line.
        let n = ((x1 - x0) / step).ceil() as usize + 4;
        let pts: Vec<P> = (0..=n)
            .map(|q| {
                let x = x0 - 2.0 * step + q as f64 * step;
                (x, base + sag(x))
            })
            .collect();
        c.curve(&pts, false);
        // Joints square to the course, about every 10 cm along it, up to
        // the next course.
        let mut x = x0 - 0.2 + 0.1 * noise(j, 0, 1900);
        while x < x1 + 0.2 {
            let m = slope(x);
            let len = (1.0 + m * m).sqrt();
            let (nx, ny) = (-m / len, 1.0 / len);
            let from = (x, base + sag(x));
            // How far along the normal the next course is.
            let mut t = h;
            for _ in 0..4 {
                let q = (from.0 + nx * t, from.1 + ny * t);
                let gap = base + h + sag(q.0) - q.1;
                t += gap * ny;
            }
            let lean = 0.08 * (at_noise(from, 1902) - 0.5);
            let to = (from.0 + (nx + lean * ny) * t, from.1 + (ny - lean * nx) * t);
            c.raw(from, to);
            x += lerp(0.085, 0.115, at_noise(from, 1901)) / len;
        }
        if c.full {
            return;
        }
    }
}

/// Rubble walling: stones of all sizes roughly in courses, rounded and
/// irregular, with small pinning stones in some joints.
fn rubble(c: &mut Canvas) {
    let (x0, y0, x1, y1) = c.area;
    let gap = 0.012;
    for (b0, b1, row) in courses(y0, y1, 0.14, 0.34, 1.8, 2000) {
        for p in pieces(row, x0, x1, 0.18, 0.55, 2.2, 2010) {
            if !c.near(p.x0, b0 - 0.05, p.x1, b1 + 0.05) {
                continue;
            }
            let r = |k: u64| noise(p.id.0, p.id.1, 2020 + k);
            let (a0, a1) = (p.x0 + gap, p.x1 - gap);
            // Each stone a little lower or higher than its course, its top
            // and bottom a little sloped.
            let lift = 0.012 * (r(0) - 0.5);
            let tilt = 0.016 * (r(1) - 0.5);
            let (c0, c1) = (b0 + gap + lift, b1 - gap + lift);
            // Eight points round the box, each moved in a little; then
            // rounded twice.
            let jig = |k: u64, amount: f64| amount * (r(k) - 0.5);
            let (w, h) = (a1 - a0, c1 - c0);
            let pts = vec![
                (a0 + w * 0.15 + jig(2, 0.02), c0 - tilt + jig(3, 0.01)),
                (a1 - w * 0.2 + jig(4, 0.02), c0 + tilt + jig(5, 0.01)),
                (a1 + jig(6, 0.015), c0 + h * 0.3 + tilt),
                (a1 + jig(7, 0.015), c1 - h * 0.25 + tilt),
                (a1 - w * 0.15 + jig(8, 0.02), c1 + tilt + jig(9, 0.01)),
                (a0 + w * 0.25 + jig(10, 0.02), c1 - tilt + jig(11, 0.01)),
                (a0 + jig(12, 0.015), c1 - h * 0.2 - tilt),
                (a0 + jig(13, 0.015), c0 + h * 0.35 - tilt),
            ];
            let round = chaikin(&chaikin(&pts));
            c.path(&round, true);
            // A pinning stone at the joint after it.
            if r(14) < 0.25 {
                let q = (p.x1, lerp(b0, b1, lerp(0.3, 0.7, r(15))));
                c.blob(q, 0.018, 0.6, r(16) * PI, 5, 0.25, 2030);
            }
            if c.full {
                return;
            }
        }
    }
}

/// A closed polygon with its corners cut (Chaikin): rounder each time.
fn chaikin(pts: &[P]) -> Vec<P> {
    let n = pts.len();
    let mut out = Vec::with_capacity(2 * n);
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        out.push((0.75 * a.0 + 0.25 * b.0, 0.75 * a.1 + 0.25 * b.1));
        out.push((0.25 * a.0 + 0.75 * b.0, 0.25 * a.1 + 0.75 * b.1));
    }
    out
}

/// The plain lines that stand for a texture where it cannot be drawn
/// piece by piece (DXF patterns, areas too large).
pub fn stand_in(p: HatchPattern) -> &'static [crate::hatch::Family] {
    use crate::hatch::Family;
    use HatchPattern::*;
    const fn f(
        angle: f64,
        origin: (f64, f64),
        delta: (f64, f64),
        dashes: &'static [f64],
    ) -> Family {
        Family {
            angle,
            origin,
            delta,
            dashes,
        }
    }
    const BOARDS_SI: &[Family] = &[
        f(0.0, (0.0, 0.0), (0.0, 0.15), &[]),
        f(90.0, (0.0, 0.0), (0.15, 0.6), &[0.15, -0.15]),
    ];
    const GRAIN_SI: &[Family] = &[f(0.0, (0.0, 0.0), (0.5, 0.05), &[0.9, -0.25, 0.3, -0.25])];
    const PANELS_SI: &[Family] = &[
        f(0.0, (0.0, 0.0), (0.0, 1.25), &[]),
        f(90.0, (0.0, 0.0), (0.0, 2.5), &[]),
    ];
    const LINES_SI: &[Family] = &[f(0.0, (0.0, 0.0), (0.0, 0.09), &[])];
    const DOTS_SI: &[Family] = &[
        f(0.0, (0.0, 0.0), (0.07, 0.13), &[0.0, -0.13]),
        f(60.0, (0.04, 0.02), (0.11, 0.23), &[0.02, -0.4]),
    ];
    const DIAMOND_SI: &[Family] = &[
        f(45.0, (0.0, 0.0), (0.05, 0.05), &[0.04, -0.03]),
        f(135.0, (0.025, 0.0), (0.05, 0.05), &[0.04, -0.03]),
    ];
    const MESH_SI: &[Family] = &[
        f(38.66, (0.0, 0.0), (0.0, 0.0625), &[]),
        f(141.34, (0.0, 0.0), (0.0, 0.0625), &[]),
    ];
    const HERRING_SI: &[Family] = &[
        f(45.0, (0.0, 0.0), (0.1414, 0.1414), &[0.2828, -0.1414]),
        f(135.0, (0.0, 0.0), (0.1414, 0.1414), &[0.2828, -0.1414]),
    ];
    const COURSES_SI: &[Family] = &[
        f(0.0, (0.0, 0.0), (0.0, 0.3), &[]),
        f(90.0, (0.0, 0.0), (0.3, 0.6), &[0.3, -0.3]),
    ];
    const PEBBLE_SI: &[Family] = &[
        f(0.0, (0.0, 0.0), (0.03, 0.05), &[0.02, -0.04]),
        f(0.0, (0.015, 0.025), (0.03, 0.05), &[0.015, -0.045]),
    ];
    match p {
        WoodBoards | WoodFinished | Pine | Oak | Maple | Birch | Ash | Walnut | Mahogany => {
            BOARDS_SI
        }
        WoodVeins => GRAIN_SI,
        MetalBrushed | ConcreteFinished => PANELS_SI,
        MetalPerforated | ConcreteRough | ConcreteBare | Terrazzo => DOTS_SI,
        DiamondPlate => DIAMOND_SI,
        Corrugated | BoardFormed => LINES_SI,
        MetalMesh => MESH_SI,
        ConcretePavers => HERRING_SI,
        Limestone | StoneBlocks | Ashlar | Rubble | Setts => COURSES_SI,
        Pebbles | RockyPath => PEBBLE_SI,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hatch::hatch_lines;
    use archisim_core::{Hatch, Polyline};

    fn hatch(p: HatchPattern, size: f64) -> Hatch {
        Hatch {
            loops: vec![Polyline::straight(
                vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(size, 0.0),
                    Point2::new(size, size),
                    Point2::new(0.0, size),
                ],
                true,
            )],
            pattern: p,
            scale: 1.0,
            angle: 0.0,
            background: None,
            opacity: 1.0,
        }
    }

    /// The pieces depend on where they are, not on the boundary: a smaller
    /// boundary inside a larger one draws the same lines there.
    #[test]
    fn a_piece_is_the_same_wherever_the_boundary_goes() {
        for p in [
            HatchPattern::Pine,
            HatchPattern::Rubble,
            HatchPattern::Pebbles,
        ] {
            let small = hatch_lines(&hatch(p, 1.0), 0.05).unwrap();
            let large = hatch_lines(&hatch(p, 2.0), 0.05).unwrap();
            let inner: Vec<_> = large
                .iter()
                .filter(|(a, b)| a.x < 0.95 && a.y < 0.95 && b.x < 0.95 && b.y < 0.95)
                .collect();
            assert!(!inner.is_empty(), "{p:?}");
            for (a, b) in inner {
                assert!(
                    small
                        .iter()
                        .any(|(c, d)| c.approx_eq(*a) && d.approx_eq(*b)),
                    "{p:?}: {a:?} {b:?}"
                );
            }
        }
    }

    /// A texture too large to draw piece by piece is drawn with its plain
    /// lines, not as a solid fill.
    #[test]
    fn a_huge_area_falls_back_to_plain_lines() {
        let h = hatch(HatchPattern::Terrazzo, 40.0);
        let lines = hatch_lines(&h, 0.5).unwrap();
        assert!(!lines.is_empty());
        assert!(lines.len() < LIMIT);
        // Every texture has its plain lines (DXF patterns).
        for p in HatchPattern::ALL {
            if is_texture(p) {
                assert!(!stand_in(p).is_empty(), "{p:?}");
                assert!(p.is_model_scale() && p.is_presentation(), "{p:?}");
            }
        }
    }

    /// Turned, the texture turns with the hatch: the boards of a wood at
    /// 90° run up the page.
    #[test]
    fn the_texture_turns_with_the_hatch() {
        let mut h = hatch(HatchPattern::WoodFinished, 3.0);
        let along = |lines: &[(Point2, Point2)]| {
            let (mut x, mut y) = (0.0, 0.0);
            for (a, b) in lines {
                x += (b.x - a.x).abs();
                y += (b.y - a.y).abs();
            }
            x / y
        };
        assert!(along(&hatch_lines(&h, 0.05).unwrap()) > 3.0);
        h.angle = std::f64::consts::FRAC_PI_2;
        assert!(along(&hatch_lines(&h, 0.05).unwrap()) < 0.33);
    }
}
