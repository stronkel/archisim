//! The shape of an opening in elevation: a rectangle, an arch over two
//! jambs (round, segmental, pointed, elliptical) or an oculus.
//!
//! In the family's axes: `x` across the opening, from `-w / 2` to `w / 2`,
//! and `z` up from the sill, from 0 to `h`. Every shape is convex, so its
//! outline, the outline set in from it (a frame's inner edge) and the band
//! between the two cut into convex pieces, which is what the `outline`
//! statement of a recipe builds and what cuts the wall.

use std::f64::consts::PI;

/// How the top of an opening is shaped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShapeKind {
    /// Square head (dintel recto).
    #[default]
    Rect,
    /// Semicircular arch (medio punto): its spring `w / 2` below the top.
    Round,
    /// Segmental arch (rebajado): an arc of a circle rising `rise` above
    /// its spring.
    Segmental,
    /// Pointed arch (apuntado, ojival): two arcs meeting at the top,
    /// `rise` above the spring (equilateral when `rise` is `w·√3/2`).
    Pointed,
    /// Elliptical arch (carpanel): half an ellipse `rise` high.
    Elliptic,
    /// An oculus: an ellipse (a circle when square) filling the box.
    Oculus,
}

impl ShapeKind {
    /// The names a recipe uses, in Spanish, in order.
    pub const NAMES: [(&'static str, ShapeKind); 6] = [
        ("Recto", ShapeKind::Rect),
        ("Medio punto", ShapeKind::Round),
        ("Rebajado", ShapeKind::Segmental),
        ("Apuntado", ShapeKind::Pointed),
        ("Carpanel", ShapeKind::Elliptic),
        ("Óculo", ShapeKind::Oculus),
    ];

    /// The kind called `name` (in Spanish as a recipe offers it, or in
    /// English: `rect`, `round`, `segmental`, `pointed`, `elliptic`,
    /// `oculus`).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::NAMES
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, k)| *k)
            .or(match name {
                "rect" => Some(Self::Rect),
                "round" => Some(Self::Round),
                "segmental" => Some(Self::Segmental),
                "pointed" => Some(Self::Pointed),
                "elliptic" => Some(Self::Elliptic),
                "oculus" => Some(Self::Oculus),
                _ => None,
            })
    }

    pub fn name(self) -> &'static str {
        Self::NAMES
            .iter()
            .find(|(_, k)| *k == self)
            .map_or("Recto", |(n, _)| n)
    }
}

/// An opening's shape: its kind and, for the arches that take one, how
/// far the arch rises above its spring (metres).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OpeningShape {
    pub kind: ShapeKind,
    pub rise: f64,
}

/// Segments of a half circle: enough for an arch of a few metres to look
/// round, few enough for thousands of windows.
const ARC_STEPS: usize = 24;

impl OpeningShape {
    pub const RECT: OpeningShape = OpeningShape {
        kind: ShapeKind::Rect,
        rise: 0.0,
    };

    pub fn new(kind: ShapeKind, rise: f64) -> Self {
        Self { kind, rise }
    }

    pub fn is_rect(&self) -> bool {
        self.kind == ShapeKind::Rect
    }

    /// How far the arch rises above its spring in an opening `w` wide and
    /// `h` high (0 for a square head; the whole height for an oculus).
    pub fn arch_rise(&self, w: f64, h: f64) -> f64 {
        let r = match self.kind {
            ShapeKind::Rect => 0.0,
            ShapeKind::Round => w / 2.0,
            ShapeKind::Segmental => self.rise.clamp(0.01, w / 2.0),
            ShapeKind::Pointed => self.rise.max(w / 2.0),
            ShapeKind::Elliptic => self.rise.max(0.01),
            ShapeKind::Oculus => h,
        };
        r.min(h)
    }

    /// Height of the spring (where the arch starts over the jambs) above
    /// the sill.
    pub fn spring(&self, w: f64, h: f64) -> f64 {
        match self.kind {
            ShapeKind::Oculus => h / 2.0,
            _ => h - self.arch_rise(w, h),
        }
    }

    /// The top of the opening, from the right spring to the left one (or,
    /// for a square head, the right top corner to the left one),
    /// counter-clockwise seen with `x` to the right and `z` up.
    fn head(&self, w: f64, h: f64) -> Vec<[f64; 2]> {
        let hw = w / 2.0;
        let f = self.arch_rise(w, h);
        let zs = h - f;
        let arc = |cx: f64, cz: f64, r: f64, a0: f64, a1: f64, steps: usize| {
            (0..=steps)
                .map(|i| {
                    let a = a0 + (a1 - a0) * i as f64 / steps as f64;
                    [cx + r * a.cos(), cz + r * a.sin()]
                })
                .collect::<Vec<_>>()
        };
        match self.kind {
            ShapeKind::Rect => vec![[hw, h], [-hw, h]],
            ShapeKind::Round => arc(0.0, zs, hw, 0.0, PI, ARC_STEPS),
            ShapeKind::Segmental => {
                // The circle through both springs and the crown.
                let r = (hw * hw + f * f) / (2.0 * f);
                let a0 = (zs - (h - r)).atan2(hw);
                // An even number of steps, so that the crown is one.
                let steps =
                    ((ARC_STEPS as f64 * (PI - 2.0 * a0) / PI / 2.0).ceil() as usize).max(2) * 2;
                arc(0.0, h - r, r, a0, PI - a0, steps)
            }
            ShapeKind::Pointed => {
                // Two arcs, each centred on the spring line on the far
                // side of the middle, meeting at the crown.
                let cx = (hw * hw - f * f) / w;
                let r = hw - cx;
                let top = f.atan2(-cx);
                let mut pts = arc(cx, zs, r, 0.0, top, ARC_STEPS / 2);
                let left = arc(-cx, zs, r, PI - top, PI, ARC_STEPS / 2);
                pts.extend(left.into_iter().skip(1));
                pts
            }
            ShapeKind::Elliptic => (0..=ARC_STEPS)
                .map(|i| {
                    let a = PI * i as f64 / ARC_STEPS as f64;
                    [hw * a.cos(), zs + f * a.sin()]
                })
                .collect(),
            ShapeKind::Oculus => (0..=ARC_STEPS)
                .map(|i| {
                    let a = PI * i as f64 / ARC_STEPS as f64;
                    [hw * a.cos(), h / 2.0 + h / 2.0 * a.sin()]
                })
                .collect(),
        }
    }

    /// The outline of an opening `w` wide and `h` high: a convex polygon,
    /// counter-clockwise (x to the right, z up), starting at the bottom
    /// left corner (for an oculus, at its lowest point).
    pub fn polygon(&self, w: f64, h: f64) -> Vec<[f64; 2]> {
        let hw = w / 2.0;
        if self.kind == ShapeKind::Oculus {
            let steps = 2 * ARC_STEPS;
            return (0..steps)
                .map(|i| {
                    let a = -PI / 2.0 + 2.0 * PI * i as f64 / steps as f64;
                    [hw * a.cos(), h / 2.0 + h / 2.0 * a.sin()]
                })
                .collect();
        }
        let mut pts = vec![[-hw, 0.0], [hw, 0.0]];
        let head = self.head(w, h);
        pts.extend(head);
        dedup(&mut pts);
        pts
    }

    /// What closes the opening's box above the opening: the top of the
    /// shape from left to right (x growing). The wall fills between it
    /// and the top of the box.
    pub fn upper(&self, w: f64, h: f64) -> Vec<[f64; 2]> {
        let mut head = self.head(w, h);
        head.reverse();
        head
    }

    /// The bottom of the shape from left to right; the wall fills between
    /// the bottom of the box and it. Only an oculus has one that is not
    /// the sill.
    pub fn lower(&self, w: f64, h: f64) -> Vec<[f64; 2]> {
        let hw = w / 2.0;
        if self.kind == ShapeKind::Oculus {
            (0..=ARC_STEPS)
                .map(|i| {
                    let a = PI + PI * i as f64 / ARC_STEPS as f64;
                    [hw * a.cos(), h / 2.0 + h / 2.0 * a.sin()]
                })
                .collect()
        } else {
            vec![[-hw, 0.0], [hw, 0.0]]
        }
    }
}

fn dedup(pts: &mut Vec<[f64; 2]>) {
    pts.dedup_by(|a, b| (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-9);
    while pts.len() > 2 {
        let (f, l) = (pts[0], pts[pts.len() - 1]);
        if (f[0] - l[0]).hypot(f[1] - l[1]) < 1e-9 {
            pts.pop();
        } else {
            break;
        }
    }
}

/// The convex polygon `poly` (counter-clockwise) set in by `d` (out when
/// negative): every edge moved inwards and each corner where two moved
/// edges meet. Keeps one point per point of `poly`, so a band between a
/// polygon and its inset pairs them up.
pub fn inset(poly: &[[f64; 2]], d: f64) -> Vec<[f64; 2]> {
    let n = poly.len();
    if n < 3 || d == 0.0 {
        return poly.to_vec();
    }
    // Each edge's inward normal (to the left, counter-clockwise).
    let normal = |i: usize| {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l = dx.hypot(dy).max(1e-12);
        [-dy / l, dx / l]
    };
    (0..n)
        .map(|i| {
            let prev = normal((i + n - 1) % n);
            let next = normal(i);
            // The point moved along both normals: p + d·m with m·nₖ = 1.
            let s = [prev[0] + next[0], prev[1] + next[1]];
            let dot = 1.0 + prev[0] * next[0] + prev[1] * next[1];
            let k = if dot.abs() < 1e-9 { 0.0 } else { d / dot };
            [poly[i][0] + s[0] * k, poly[i][1] + s[1] * k]
        })
        .collect()
}

/// The convex polygon `poly` cut to the box `lo`..`hi` (empty when it
/// misses it).
pub fn clip(poly: &[[f64; 2]], lo: [f64; 2], hi: [f64; 2]) -> Vec<[f64; 2]> {
    let mut out = poly.to_vec();
    // (axis, limit, keep above)
    for (axis, limit, above) in [
        (0, lo[0], true),
        (0, hi[0], false),
        (1, lo[1], true),
        (1, hi[1], false),
    ] {
        if out.is_empty() {
            break;
        }
        let inside = |p: &[f64; 2]| {
            if above {
                p[axis] >= limit - 1e-12
            } else {
                p[axis] <= limit + 1e-12
            }
        };
        let mut next = Vec::with_capacity(out.len() + 2);
        for i in 0..out.len() {
            let a = out[i];
            let b = out[(i + 1) % out.len()];
            let (ia, ib) = (inside(&a), inside(&b));
            if ia {
                next.push(a);
            }
            if ia != ib {
                let t = (limit - a[axis]) / (b[axis] - a[axis]);
                next.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            }
        }
        out = next;
    }
    dedup(&mut out);
    if out.len() < 3 || area(&out) < 1e-10 {
        Vec::new()
    } else {
        out
    }
}

/// Signed area (positive counter-clockwise).
pub fn area(poly: &[[f64; 2]]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

/// The band between the outline set in by `a` and set in by `b` (`a` <
/// `b`), as convex quadrilaterals, one per edge of the outline.
pub fn band(poly: &[[f64; 2]], a: f64, b: f64) -> Vec<Vec<[f64; 2]>> {
    let (outer, inner) = (inset(poly, a), inset(poly, b));
    let n = outer.len();
    (0..n)
        .filter_map(|i| {
            let j = (i + 1) % n;
            let q = vec![outer[i], outer[j], inner[j], inner[i]];
            (area(&q) > 1e-10).then_some(q)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds() -> Vec<OpeningShape> {
        vec![
            OpeningShape::RECT,
            OpeningShape::new(ShapeKind::Round, 0.0),
            OpeningShape::new(ShapeKind::Segmental, 0.25),
            OpeningShape::new(ShapeKind::Pointed, 0.9),
            OpeningShape::new(ShapeKind::Elliptic, 0.35),
            OpeningShape::new(ShapeKind::Oculus, 0.0),
        ]
    }

    #[test]
    fn outlines_are_convex_and_fill_their_box() {
        for s in kinds() {
            let (w, h) = (
                1.2,
                if s.kind == ShapeKind::Oculus {
                    0.8
                } else {
                    2.4
                },
            );
            let p = s.polygon(w, h);
            assert!(area(&p) > 0.0, "{s:?} counter-clockwise");
            let n = p.len();
            for i in 0..n {
                let (a, b, c) = (p[i], p[(i + 1) % n], p[(i + 2) % n]);
                let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
                assert!(cross > -1e-9, "{s:?} convex at {i}");
            }
            for (k, lim) in [(0, w / 2.0), (1, h)] {
                let max = p.iter().map(|q| q[k]).fold(f64::MIN, f64::max);
                assert!(
                    (max - lim).abs() < 1e-6,
                    "{s:?} reaches {lim} on {k}: {max}"
                );
            }
            // The box less the wall filled above and below is the shape.
            let up = s.upper(w, h);
            let lo = s.lower(w, h);
            let between = |line: &[[f64; 2]], to: f64| -> f64 {
                line.windows(2)
                    .map(|q| (q[1][0] - q[0][0]) * ((to - q[0][1]) + (to - q[1][1])) / 2.0)
                    .sum::<f64>()
                    .abs()
            };
            let filled = between(&up, h) + between(&lo, 0.0);
            assert!((w * h - filled - area(&p)).abs() < 1e-6, "{s:?}");
        }
    }

    #[test]
    fn insets_and_bands() {
        for s in kinds() {
            let p = s.polygon(1.0, 2.0);
            let q = inset(&p, 0.05);
            assert_eq!(p.len(), q.len());
            assert!(area(&q) < area(&p));
            let ring: f64 = band(&p, 0.0, 0.05).iter().map(|q| area(q)).sum();
            assert!((ring - (area(&p) - area(&q))).abs() < 1e-6, "{s:?}");
            // Cut to the part above the spring.
            let top = clip(&q, [-1.0, s.spring(1.0, 2.0)], [1.0, 3.0]);
            assert!(top.is_empty() || area(&top) > 0.0);
        }
        let sq = OpeningShape::RECT.polygon(1.0, 1.0);
        let i = inset(&sq, 0.1);
        assert!((area(&i) - 0.64).abs() < 1e-9);
        assert!((area(&inset(&sq, -0.1)) - 1.44).abs() < 1e-9);
    }

    #[test]
    fn pointed_and_round_springs() {
        let r = OpeningShape::new(ShapeKind::Round, 0.0);
        assert!((r.spring(1.0, 2.0) - 1.5).abs() < 1e-12);
        let eq = OpeningShape::new(ShapeKind::Pointed, 3f64.sqrt() / 2.0);
        assert!((eq.spring(1.0, 2.0) - (2.0 - 3f64.sqrt() / 2.0)).abs() < 1e-12);
        let top = eq
            .polygon(1.0, 2.0)
            .iter()
            .map(|q| q[1])
            .fold(0.0, f64::max);
        assert!((top - 2.0).abs() < 1e-9);
    }
}
