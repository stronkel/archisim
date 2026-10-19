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
    }
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
    for f in families(h.pattern) {
        clip_family(&polys, f, unit, h.angle, &mut out)?;
    }
    Some(out)
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
        for pair in xs.chunks_exact(2) {
            let (s0, s1) = (pair[0], pair[1]);
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
