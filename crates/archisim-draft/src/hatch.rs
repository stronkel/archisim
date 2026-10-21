//! Hatch patterns as families of parallel dashed lines (like AutoCAD's
//! `.pat` files, but ArchiSim's own patterns), and the lines of a hatch
//! clipped to its boundaries.

use archisim_core::{Hatch, HatchPattern, Point2};

use crate::seg::dir;
use crate::{loop_polygon, CURVE_TOL};

/// A family of parallel lines of a pattern. Lengths in paper millimetres,
/// or in metres for patterns at real size ([`HatchPattern::is_model_scale`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Family {
    /// Direction of the lines, degrees.
    pub angle: f64,
    /// A point the first line passes through.
    pub origin: (f64, f64),
    /// From one line to the next: shift along the lines, and spacing
    /// across them.
    pub delta: (f64, f64),
    /// Dash pattern: positive dashes, negative gaps, zero dots; empty for a
    /// continuous line.
    pub dashes: &'static [f64],
}

const fn fam(angle: f64, origin: (f64, f64), delta: (f64, f64), dashes: &'static [f64]) -> Family {
    Family {
        angle,
        origin,
        delta,
        dashes,
    }
}

/// The line families of a pattern (none for a solid fill).
pub fn families(p: HatchPattern) -> &'static [Family] {
    const LINES: &[Family] = &[fam(0.0, (0.0, 0.0), (0.0, 2.0), &[])];
    const GRID: &[Family] = &[
        fam(0.0, (0.0, 0.0), (0.0, 2.5), &[]),
        fam(90.0, (0.0, 0.0), (0.0, 2.5), &[]),
    ];
    const DIAGONAL: &[Family] = &[fam(45.0, (0.0, 0.0), (0.0, 1.5), &[])];
    const DIAGONAL_CROSS: &[Family] = &[
        fam(45.0, (0.0, 0.0), (0.0, 2.0), &[]),
        fam(135.0, (0.0, 0.0), (0.0, 2.0), &[]),
    ];
    // Facing brick 24 × 5 cm with 1 cm joints, stretcher bond.
    const BRICK: &[Family] = &[
        fam(0.0, (0.0, 0.0), (0.0, 0.06), &[]),
        fam(90.0, (0.0, 0.0), (0.06, 0.125), &[0.06, -0.06]),
    ];
    const CONCRETE: &[Family] = &[
        fam(0.0, (0.0, 0.0), (0.9, 1.7), &[0.0, -1.8]),
        fam(60.0, (0.3, 0.2), (1.1, 2.3), &[0.45, -3.0]),
        fam(150.0, (0.8, 0.6), (1.3, 2.6), &[0.35, -3.4]),
    ];
    const DOTS: &[Family] = &[fam(0.0, (0.0, 0.0), (0.6, 1.2), &[0.0, -1.2])];
    const GRAVEL: &[Family] = &[
        fam(0.0, (0.0, 0.0), (1.2, 2.1), &[0.5, -2.0]),
        fam(60.0, (0.25, 0.0), (1.2, 2.1), &[0.5, -2.0]),
        fam(120.0, (0.5, 0.0), (1.2, 2.1), &[0.5, -2.0]),
    ];
    // Floor tiles 30 × 30 cm.
    const TILES: &[Family] = &[
        fam(0.0, (0.0, 0.0), (0.0, 0.3), &[]),
        fam(90.0, (0.0, 0.0), (0.0, 0.3), &[]),
    ];
    const EARTH: &[Family] = &[
        fam(45.0, (0.0, 0.0), (2.0, 0.6), &[4.0, -4.0]),
        fam(45.0, (0.0, 0.0), (2.0, 0.6), &[0.0, -8.0]),
    ];
    const GRASS: &[Family] = &[
        fam(90.0, (0.0, 0.0), (1.5, 1.8), &[0.8, -3.0]),
        fam(70.0, (0.3, 0.0), (1.5, 1.8), &[0.7, -3.1]),
        fam(110.0, (-0.3, 0.0), (1.5, 1.8), &[0.7, -3.1]),
    ];
    const WOOD: &[Family] = &[fam(0.0, (0.0, 0.0), (3.0, 1.2), &[10.0, -0.8, 2.5, -0.8])];
    // Setts 20 × 10 cm and slabs 60 × 40 cm, stretcher bond.
    const COBBLES: &[Family] = &[
        fam(0.0, (0.0, 0.0), (0.0, 0.1), &[]),
        fam(90.0, (0.0, 0.0), (0.1, 0.1), &[0.1, -0.1]),
    ];
    const SLABS: &[Family] = &[
        fam(0.0, (0.0, 0.0), (0.0, 0.4), &[]),
        fam(90.0, (0.0, 0.0), (0.4, 0.3), &[0.4, -0.4]),
    ];
    // Boards 14,5 cm with joints every 1,2 m, shifted 40 cm board to board.
    const DECKING: &[Family] = &[
        fam(0.0, (0.0, 0.0), (0.0, 0.145), &[]),
        fam(90.0, (0.0, 0.0), (0.145, 0.4), &[0.145, -1.015]),
    ];
    // Stand-ins of the patterns drawn by `procedural` (for DXF).
    const STONE: &[Family] = &[
        fam(0.0, (0.0, 0.0), (0.17, 0.4), &[0.33, -0.07]),
        fam(80.0, (0.0, 0.0), (0.13, 0.43), &[0.36, -0.05]),
        fam(105.0, (0.21, 0.0), (0.11, 0.47), &[0.31, -0.09]),
    ];
    const WOOD_GRAIN: &[Family] = &[fam(0.0, (0.0, 0.0), (3.7, 1.2), &[14.0, -1.0, 4.0, -1.0])];
    const GRAVEL_DOTS: &[Family] = &[
        fam(0.0, (0.0, 0.0), (0.45, 1.0), &[0.0, -1.0]),
        fam(0.0, (0.5, 0.33), (0.7, 1.0), &[0.0, -1.7]),
    ];
    const LAWN_STRIPES: &[Family] = &[fam(0.0, (0.0, 0.0), (0.0, 3.0), &[])];
    const LAWN_DOTS: &[Family] = &[
        fam(0.0, (0.0, 0.0), (0.3, 0.8), &[0.0, -0.8]),
        fam(0.0, (0.4, 0.4), (0.5, 0.8), &[0.0, -1.3]),
    ];
    const WATER: &[Family] = &[fam(0.0, (0.0, 0.0), (2.3, 1.6), &[1.6, -3.4])];
    match p {
        HatchPattern::Solid => &[],
        HatchPattern::Lines => LINES,
        HatchPattern::Grid => GRID,
        HatchPattern::Diagonal => DIAGONAL,
        HatchPattern::DiagonalCross => DIAGONAL_CROSS,
        HatchPattern::Brick => BRICK,
        HatchPattern::Concrete => CONCRETE,
        HatchPattern::Dots => DOTS,
        HatchPattern::Gravel => GRAVEL,
        HatchPattern::Tiles => TILES,
        HatchPattern::Earth => EARTH,
        HatchPattern::Grass => GRASS,
        HatchPattern::Wood => WOOD,
        HatchPattern::Cobbles => COBBLES,
        HatchPattern::Slabs => SLABS,
        HatchPattern::Stone => STONE,
        HatchPattern::Decking => DECKING,
        HatchPattern::WoodGrain => WOOD_GRAIN,
        HatchPattern::GravelDots => GRAVEL_DOTS,
        HatchPattern::LawnStripes => LAWN_STRIPES,
        HatchPattern::LawnDots => LAWN_DOTS,
        HatchPattern::Water => WATER,
    }
}

/// The filled bands of a pattern drawn in bands (mown lawn: 1,5 m bands
/// of the pattern colour, 1,5 m apart), as rings to fill with the
/// even-odd rule; empty for other patterns. Each boundary is cut to each
/// band on its own: even-odd, the pieces fill the band inside the hatch.
pub fn hatch_bands(h: &Hatch) -> Vec<Vec<Point2>> {
    if h.pattern != HatchPattern::LawnStripes {
        return Vec::new();
    }
    let polys = hatch_polygons(h);
    let (band, period) = (1.5 * h.scale, 3.0 * h.scale);
    if band <= 1e-9 {
        return Vec::new();
    }
    let n = dir(h.angle).perp_left();
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in polys.iter().flatten() {
        lo = lo.min(p.dot(n));
        hi = hi.max(p.dot(n));
    }
    if !lo.is_finite() || (hi - lo) / period > MAX_HATCH_LINES as f64 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for k in (lo / period).floor() as i64..=(hi / period).ceil() as i64 {
        let (c0, c1) = (k as f64 * period, k as f64 * period + band);
        for poly in &polys {
            let cut = half_plane(&half_plane(poly, n, c0), n * -1.0, -c1);
            if cut.len() >= 3 {
                out.push(cut);
            }
        }
    }
    out
}

/// The part of a polygon where `p·n >= c` (Sutherland–Hodgman).
fn half_plane(poly: &[Point2], n: Point2, c: f64) -> Vec<Point2> {
    let mut out = Vec::new();
    let m = poly.len();
    for i in 0..m {
        let (a, b) = (poly[i], poly[(i + 1) % m]);
        let (da, db) = (a.dot(n) - c, b.dot(n) - c);
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            out.push(a + (b - a) * (da / (da - db)));
        }
    }
    out
}

/// Most lines a hatch draws; a denser one is drawn as a plain fill (the
/// pattern would be a grey blur anyway).
pub const MAX_HATCH_LINES: usize = 40_000;

/// The polygons of a hatch's boundaries.
pub fn hatch_polygons(h: &Hatch) -> Vec<Vec<Point2>> {
    h.loops
        .iter()
        .map(|l| loop_polygon(l, CURVE_TOL))
        .filter(|p| p.len() >= 3)
        .collect()
}

/// The pattern lines of a hatch, clipped to its boundaries (even-odd), as
/// segments in drawing units (a dot is a segment of zero length). `k`
/// turns paper millimetres into drawing units. `None` if the pattern is
/// too dense to draw line by line.
pub fn hatch_lines(h: &Hatch, k: f64) -> Option<Vec<(Point2, Point2)>> {
    let polys = hatch_polygons(h);
    let unit = if h.pattern.is_model_scale() {
        h.scale
    } else {
        k * h.scale
    };
    let mut out = Vec::new();
    if h.pattern == HatchPattern::LawnStripes {
        // Drawn as filled bands (`hatch_bands`).
        return Some(out);
    }
    if procedural(h.pattern) {
        draw_procedural(&polys, h.pattern, unit, h.angle, &mut out)?;
        return Some(out);
    }
    for f in families(h.pattern) {
        clip_family(&polys, f, unit, h.angle, &mut out)?;
    }
    Some(out)
}

/// Patterns drawn piece by piece, each piece a little different (stones,
/// grain, dots), rather than as repeating lines.
pub fn procedural(p: HatchPattern) -> bool {
    matches!(
        p,
        HatchPattern::Stone
            | HatchPattern::WoodGrain
            | HatchPattern::GravelDots
            | HatchPattern::LawnDots
    )
}

/// A number in [0, 1) that is always the same for the same cell.
fn noise(i: i64, j: i64, k: u64) -> f64 {
    let mut x = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (j as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ k.wrapping_mul(0x1656_67B1_9E37_79F9);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    x ^= x >> 33;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

/// The pieces of a procedural pattern over the polygons, clipped to them.
fn draw_procedural(
    polys: &[Vec<Point2>],
    p: HatchPattern,
    unit: f64,
    turn: f64,
    out: &mut Vec<(Point2, Point2)>,
) -> Option<()> {
    // The pattern's own axes, turned with the hatch.
    let u = dir(turn);
    let v = u.perp_left();
    let (mut x0, mut x1, mut y0, mut y1) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for q in polys.iter().flatten() {
        let (x, y) = (q.dot(u), q.dot(v));
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
        y1 = y1.max(y);
    }
    if !x0.is_finite() || unit <= 0.0 {
        return Some(());
    }
    let at = |x: f64, y: f64| u * x + v * y;
    let cell = match p {
        HatchPattern::Stone => 0.4,
        HatchPattern::WoodGrain => 1.2,
        HatchPattern::GravelDots => 0.9,
        _ => 0.7,
    } * unit;
    let (i0, i1) = (
        (x0 / cell).floor() as i64 - 1,
        (x1 / cell).ceil() as i64 + 1,
    );
    let (j0, j1) = (
        (y0 / cell).floor() as i64 - 1,
        (y1 / cell).ceil() as i64 + 1,
    );
    if ((i1 - i0) as f64) * ((j1 - j0) as f64) > (MAX_HATCH_LINES * 2) as f64 {
        return None;
    }
    let piece = |a: Point2, b: Point2, out: &mut Vec<(Point2, Point2)>| {
        clip_segment(polys, a, b, out);
    };
    match p {
        HatchPattern::Stone => {
            // Corners of a grid, each moved at random: four-sided stones of
            // different shapes; some split in two.
            let corner = |i: i64, j: i64| {
                let x = (i as f64 + 0.36 * (noise(i, j, 1) - 0.5) * 2.0) * cell;
                let y = (j as f64 + 0.36 * (noise(i, j, 2) - 0.5) * 2.0) * cell;
                at(x, y)
            };
            for i in i0..i1 {
                for j in j0..j1 {
                    let c = corner(i, j);
                    piece(c, corner(i + 1, j), out);
                    piece(c, corner(i, j + 1), out);
                    if noise(i, j, 3) < 0.3 {
                        piece(c, corner(i + 1, j + 1), out);
                    } else if noise(i, j, 3) > 0.85 {
                        piece(corner(i + 1, j), corner(i, j + 1), out);
                    }
                }
                if out.len() > MAX_HATCH_LINES * 4 {
                    return None;
                }
            }
        }
        HatchPattern::WoodGrain => {
            // Wavy lines, each with its own wave; now and then a knot.
            let step = 0.6 * unit;
            for j in j0..j1 {
                let y = j as f64 * cell;
                let amp = (0.15 + 0.25 * noise(0, j, 4)) * unit;
                let length = (9.0 + 9.0 * noise(0, j, 5)) * unit;
                let phase = noise(0, j, 6) * std::f64::consts::TAU;
                let n = ((x1 - x0) / step).ceil() as i64 + 4;
                let wave = |x: f64| y + amp * (x / length * std::f64::consts::TAU + phase).sin();
                let mut prev = at(x0 - step, wave(x0 - step));
                for k in 0..n {
                    let x = x0 + k as f64 * step;
                    let q = at(x, wave(x));
                    piece(prev, q, out);
                    prev = q;
                }
                if noise(0, j, 7) < 0.15 {
                    // A knot: a small ellipse between this line and the next.
                    let cx = x0 + noise(0, j, 8) * (x1 - x0);
                    let (rx, ry) = (1.2 * unit, 0.35 * unit);
                    let mut prev = at(cx + rx, y + cell / 2.0);
                    for k in 1..=16 {
                        let a = k as f64 / 16.0 * std::f64::consts::TAU;
                        let q = at(cx + rx * a.cos(), y + cell / 2.0 + ry * a.sin());
                        piece(prev, q, out);
                        prev = q;
                    }
                }
                if out.len() > MAX_HATCH_LINES * 4 {
                    return None;
                }
            }
        }
        _ => {
            // Dots scattered over a grid: one, two or none per cell.
            let (k_extra, presence) = if p == HatchPattern::GravelDots {
                (0.35, 0.95)
            } else {
                (0.15, 0.7)
            };
            for i in i0..i1 {
                for j in j0..j1 {
                    for (n, wanted) in [(0u64, presence), (10, k_extra)] {
                        if noise(i, j, 20 + n) >= wanted {
                            continue;
                        }
                        let x = (i as f64 + noise(i, j, 21 + n)) * cell;
                        let y = (j as f64 + noise(i, j, 22 + n)) * cell;
                        let q = at(x, y);
                        if inside(polys, q) {
                            out.push((q, q));
                        }
                    }
                }
                if out.len() > MAX_HATCH_LINES * 4 {
                    return None;
                }
            }
        }
    }
    Some(())
}

/// Whether a point is inside the polygons (even-odd).
fn inside(polys: &[Vec<Point2>], q: Point2) -> bool {
    let mut odd = false;
    for poly in polys {
        let m = poly.len();
        for j in 0..m {
            let (a, b) = (poly[j], poly[(j + 1) % m]);
            if (a.y > q.y) != (b.y > q.y) {
                let x = a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x);
                if x > q.x {
                    odd = !odd;
                }
            }
        }
    }
    odd
}

/// The parts of the segment `a`–`b` inside the polygons (even-odd).
fn clip_segment(polys: &[Vec<Point2>], a: Point2, b: Point2, out: &mut Vec<(Point2, Point2)>) {
    let d = b - a;
    let mut ts = vec![0.0, 1.0];
    for poly in polys {
        let m = poly.len();
        for j in 0..m {
            let (p, q) = (poly[j], poly[(j + 1) % m]);
            let e = q - p;
            let den = d.cross(e);
            if den.abs() < 1e-15 {
                continue;
            }
            let t = (p - a).cross(e) / den;
            let s = (p - a).cross(d) / den;
            if t > 0.0 && t < 1.0 && (0.0..=1.0).contains(&s) {
                ts.push(t);
            }
        }
    }
    ts.sort_by(f64::total_cmp);
    for w in ts.windows(2) {
        if w[1] - w[0] < 1e-12 {
            continue;
        }
        let mid = a + d * ((w[0] + w[1]) / 2.0);
        if inside(polys, mid) {
            out.push((a + d * w[0], a + d * w[1]));
        }
    }
}

fn clip_family(
    polys: &[Vec<Point2>],
    f: &Family,
    unit: f64,
    turn: f64,
    out: &mut Vec<(Point2, Point2)>,
) -> Option<()> {
    let spacing = f.delta.1 * unit;
    if spacing.abs() < 1e-12 {
        return Some(());
    }
    let u = dir(f.angle.to_radians() + turn);
    let n = u.perp_left();
    // The origin turns with the hatch too.
    let o = {
        let (s, c) = turn.sin_cos();
        let (x, y) = (f.origin.0 * unit, f.origin.1 * unit);
        Point2::new(x * c - y * s, x * s + y * c)
    };
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in polys.iter().flatten() {
        let c = (*p - o).dot(n);
        lo = lo.min(c);
        hi = hi.max(c);
    }
    if !lo.is_finite() {
        return Some(());
    }
    let i0 = (lo / spacing).floor() as i64 - 1;
    let i1 = (hi / spacing).ceil() as i64 + 1;
    let (i0, i1) = (i0.min(i1), i0.max(i1));
    if (i1 - i0) as usize > MAX_HATCH_LINES {
        return None;
    }
    let period: f64 = f.dashes.iter().map(|d| d.abs()).sum::<f64>() * unit;
    for i in i0..=i1 {
        let c = i as f64 * spacing;
        // The line's own origin: across by `c`, shifted along by i·shift.
        let line_o = o + n * c + u * (i as f64 * f.delta.0 * unit);
        let mut xs: Vec<f64> = Vec::new();
        for poly in polys {
            let m = poly.len();
            for j in 0..m {
                let (a, b) = (poly[j], poly[(j + 1) % m]);
                let da = (a - o).dot(n) - c;
                let db = (b - o).dot(n) - c;
                if (da > 0.0) != (db > 0.0) {
                    let t = da / (da - db);
                    let q = a + (b - a) * t;
                    xs.push((q - line_o).dot(u));
                }
            }
        }
        xs.sort_by(f64::total_cmp);
        for &[s0, s1] in xs.as_chunks::<2>().0 {
            if f.dashes.is_empty() || period <= 1e-12 {
                out.push((line_o + u * s0, line_o + u * s1));
            } else {
                dashes(line_o, u, s0, s1, f.dashes, unit, period, out);
            }
            if out.len() > MAX_HATCH_LINES * 4 {
                return None;
            }
        }
    }
    Some(())
}

#[allow(clippy::too_many_arguments)]
fn dashes(
    o: Point2,
    u: Point2,
    s0: f64,
    s1: f64,
    pattern: &[f64],
    unit: f64,
    period: f64,
    out: &mut Vec<(Point2, Point2)>,
) {
    let first = (s0 / period).floor();
    let mut start = first * period;
    while start < s1 {
        let mut s = start;
        for d in pattern {
            let len = d.abs() * unit;
            if *d >= 0.0 {
                let (a, b) = (s.max(s0), (s + len).min(s1));
                if *d == 0.0 {
                    if s >= s0 && s <= s1 {
                        out.push((o + u * s, o + u * s));
                    }
                } else if b > a {
                    out.push((o + u * a, o + u * b));
                }
            }
            s += len;
        }
        start += period;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::Polyline;

    fn square(size: f64) -> Hatch {
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
            pattern: HatchPattern::Lines,
            scale: 1.0,
            angle: 0.0,
            background: None,
            opacity: 1.0,
        }
    }

    #[test]
    fn lines_fill_the_square_and_stay_inside() {
        // 1 m square at 1:50: lines 2 mm apart are 0,1 m apart.
        let h = square(1.0);
        let lines = hatch_lines(&h, 0.05).unwrap();
        assert!((9..=11).contains(&lines.len()), "{}", lines.len());
        for (a, b) in &lines {
            for p in [a, b] {
                assert!(p.x >= -1e-9 && p.x <= 1.0 + 1e-9 && p.y >= -1e-9 && p.y <= 1.0 + 1e-9);
            }
        }
    }

    #[test]
    fn holes_are_left_empty() {
        let mut h = square(1.0);
        h.loops.push(Polyline::straight(
            vec![
                Point2::new(0.25, 0.25),
                Point2::new(0.75, 0.25),
                Point2::new(0.75, 0.75),
                Point2::new(0.25, 0.75),
            ],
            true,
        ));
        h.pattern = HatchPattern::Grid;
        let lines = hatch_lines(&h, 0.05).unwrap();
        for (a, b) in lines {
            let m = (a + b) * 0.5;
            assert!(
                !(m.x > 0.26 && m.x < 0.74 && m.y > 0.26 && m.y < 0.74),
                "{m:?}"
            );
        }
    }

    #[test]
    fn presentation_patterns() {
        // Stones and dots stay inside, differ from cell to cell and fill
        // the square; lawn stripes cover about half of it.
        for p in [
            HatchPattern::Stone,
            HatchPattern::WoodGrain,
            HatchPattern::GravelDots,
            HatchPattern::LawnDots,
        ] {
            let mut h = square(4.0);
            h.pattern = p;
            let lines = hatch_lines(&h, 0.05).unwrap();
            assert!(lines.len() > 20, "{p:?} {}", lines.len());
            for (a, b) in &lines {
                for q in [a, b] {
                    assert!(q.x > -1e-9 && q.x < 4.0 + 1e-9 && q.y > -1e-9 && q.y < 4.0 + 1e-9);
                }
            }
        }
        let mut h = square(6.0);
        h.pattern = HatchPattern::LawnStripes;
        assert!(hatch_lines(&h, 0.05).unwrap().is_empty());
        let area: f64 = hatch_bands(&h)
            .iter()
            .map(|r| archisim_core::signed_area(r).abs())
            .sum();
        assert!((area - 18.0).abs() < 1e-6, "{area}");
        // Dots are not on a grid: their spacing varies.
        let mut h = square(2.0);
        h.pattern = HatchPattern::GravelDots;
        let xs: Vec<f64> = hatch_lines(&h, 0.05)
            .unwrap()
            .iter()
            .map(|(a, _)| (a.x * 1000.0).round())
            .collect();
        let distinct: std::collections::BTreeSet<i64> = xs.iter().map(|x| *x as i64).collect();
        assert!(distinct.len() > xs.len() / 2);
    }

    #[test]
    fn real_size_patterns_and_dense_ones() {
        let mut h = square(0.9);
        h.pattern = HatchPattern::Tiles;
        // 30 cm tiles in 90 cm: two inner lines each way (plus the edges).
        let lines = hatch_lines(&h, 0.05).unwrap();
        assert!(lines.len() <= 8 && lines.len() >= 4, "{}", lines.len());
        let mut dense = square(100.0);
        dense.scale = 1e-5;
        assert!(hatch_lines(&dense, 0.05).is_none());
        for p in HatchPattern::ALL {
            let mut h = square(1.0);
            h.pattern = p;
            assert!(hatch_lines(&h, 0.05).is_some(), "{p:?}");
        }
    }
}
