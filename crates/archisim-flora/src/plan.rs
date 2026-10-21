//! The plant in plan, as landscape plans draw it: from the plant itself
//! (its crown seen from above, its branches, its clumps of leaves, its
//! fronds), so the drawing and the 3D are the same plant. Each style is
//! one of the ways of drawing trees in plans; the look says whether it is
//! lines only, see-through colour, solid colour or colour without lines.

use crate::grow::{Branch, ClumpKind, FrondKind, Plant};
use crate::rng::Rng;
use crate::Foliage;
use std::f64::consts::{PI, TAU};

/// How a plant is drawn in plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanStyle {
    /// A circle and the trunk: the technical symbol.
    Circle,
    /// The crown's outline, as it is.
    Outline,
    /// The outline in small teeth.
    Toothed,
    /// The outline in rounded lobes, as a cloud.
    Cloud,
    /// Two outlines, as drawn by hand.
    Sketch,
    /// The branches seen from above, without leaves.
    Branches,
    /// Little leaves scattered over the crown.
    Leaves,
    /// Branches and some leaves.
    BranchesLeaves,
    /// The masses of leaves, with the gaps between them.
    Foliage,
    /// Dots, closer on the shaded side.
    Stipple,
    /// Lines from the trunk to the edge.
    Radial,
    /// The fronds of a palm, the leaves of a rosette.
    Fronds,
    /// Washes of colour, without lines.
    Watercolor,
}

names!(PlanStyle {
    Circle => "círculo",
    Outline => "contorno",
    Toothed => "dentado",
    Cloud => "nube",
    Sketch => "a mano",
    Branches => "ramas",
    Leaves => "hojas",
    BranchesLeaves => "ramas y hojas",
    Foliage => "follaje",
    Stipple => "punteado",
    Radial => "radial",
    Fronds => "frondas",
    Watercolor => "acuarela",
});

/// Lines, colour, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Look {
    Lines,
    SeeThrough,
    Filled,
    Colour,
}

names!(Look {
    Lines => "líneas",
    SeeThrough => "transparente",
    Filled => "rellena",
    Colour => "solo color",
});

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanOptions {
    pub style: PlanStyle,
    pub look: Look,
    /// Opacity of the colour, from 0 to 1.
    pub opacity: f64,
    /// The crown's width on paper, in mm: how much detail is worth it.
    pub crown_mm: f64,
}

impl Default for PlanOptions {
    fn default() -> Self {
        PlanOptions {
            style: PlanStyle::Outline,
            look: Look::Lines,
            opacity: 0.55,
            crown_mm: 80.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stroke {
    /// The crown's edge.
    Outline,
    /// Branches, leaves, the trunk.
    Thin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Leaves,
    Bark,
    Flowers,
    Fruit,
}

/// One mark of the drawing, in metres from the foot of the plant.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Line {
        points: Vec<[f64; 2]>,
        closed: bool,
        stroke: Stroke,
    },
    Fill {
        points: Vec<[f64; 2]>,
        tone: Tone,
        opacity: f64,
    },
}

type P2 = [f64; 2];

fn circle(c: P2, r: f64, n: usize) -> Vec<P2> {
    (0..n)
        .map(|i| {
            let a = TAU * i as f64 / n as f64;
            [c[0] + r * a.cos(), c[1] + r * a.sin()]
        })
        .collect()
}

fn inside(poly: &[P2], p: P2) -> bool {
    let mut c = false;
    let n = poly.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            c = !c;
        }
        j = i;
    }
    c
}

fn area(poly: &[P2]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

/// Corners cut once (Chaikin), for softer lines.
fn soften(poly: &[P2], closed: bool) -> Vec<P2> {
    let n = poly.len();
    if n < 3 {
        return poly.to_vec();
    }
    let mut out = Vec::with_capacity(n * 2);
    let last = if closed { n } else { n - 1 };
    if !closed {
        out.push(poly[0]);
    }
    for i in 0..last {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        out.push([a[0] * 0.75 + b[0] * 0.25, a[1] * 0.75 + b[1] * 0.25]);
        out.push([a[0] * 0.25 + b[0] * 0.75, a[1] * 0.25 + b[1] * 0.75]);
    }
    if !closed {
        out.push(poly[n - 1]);
    }
    out
}

/// Smooths values round a circle (a moving average).
fn smooth_ring(v: &[f64], half: usize) -> Vec<f64> {
    let n = v.len();
    if half == 0 || n == 0 {
        return v.to_vec();
    }
    (0..n)
        .map(|i| {
            let mut s = 0.0;
            for k in 0..=2 * half {
                s += v[(i + n + k - half) % n];
            }
            s / (2 * half + 1) as f64
        })
        .collect()
}

impl Plant {
    /// The crown seen from above: its radius round its centre at `n`
    /// equal angles.
    pub fn silhouette(&self, n: usize) -> Vec<f64> {
        let c = self.center;
        let mut r = vec![0.0_f64; n];
        let mut put = |p: P2, rad: f64| {
            let dx = p[0] - c[0];
            let dy = p[1] - c[1];
            let rho = (dx * dx + dy * dy).sqrt();
            let phi = dy.atan2(dx);
            if rho <= rad {
                // Covers the centre: reaches out all round.
                for (i, ri) in r.iter_mut().enumerate() {
                    let th = TAU * i as f64 / n as f64;
                    let ct = (th - phi).cos();
                    let st = (th - phi).sin();
                    let reach = rho * ct + (rad * rad - rho * rho * st * st).max(0.0).sqrt();
                    *ri = ri.max(reach);
                }
                return;
            }
            let half = (rad / rho).min(1.0).asin();
            let i0 = ((phi - half) / TAU * n as f64).floor() as i64;
            let i1 = ((phi + half) / TAU * n as f64).ceil() as i64;
            for i in i0..=i1 {
                let th = TAU * i as f64 / n as f64;
                let st = (th - phi).sin();
                let ct = (th - phi).cos();
                let disc = rad * rad - rho * rho * st * st;
                if disc < 0.0 || ct <= 0.0 {
                    continue;
                }
                let reach = rho * ct + disc.sqrt();
                let k = i.rem_euclid(n as i64) as usize;
                r[k] = r[k].max(reach);
            }
        };
        for cl in &self.clumps {
            if cl.kind == ClumpKind::Fruit {
                continue;
            }
            let horiz = (cl.axis[0] * cl.axis[0] + cl.axis[1] * cl.axis[1]).sqrt();
            let rad = cl.radius * (1.0 + (cl.stretch - 1.0) * horiz);
            put([cl.center[0], cl.center[1]], rad);
        }
        for f in &self.fronds {
            let m = f.rachis.len();
            for (i, p) in f.rachis.iter().enumerate() {
                let s = i as f64 / (m.max(2) - 1) as f64;
                let w = match f.kind {
                    FrondKind::Pinnate => f.width * (PI * s).sin(),
                    _ => f.width * 0.5,
                };
                put([p[0], p[1]], w.max(0.01));
            }
            if f.kind == FrondKind::Fan {
                if let Some(p) = f.rachis.last() {
                    put([p[0], p[1]], f.width);
                }
            }
        }
        if self.clumps.is_empty() && self.fronds.is_empty() {
            for b in &self.branches {
                for p in &b.points {
                    put([p[0], p[1]], 0.05);
                }
            }
        }
        // Gaps (thin crowns): bridge them from their neighbours.
        let fallback = self.species.width / 2.0;
        if r.iter().all(|v| *v <= 0.0) {
            return vec![fallback; n];
        }
        let known: Vec<usize> = (0..n).filter(|&i| r[i] > 0.0).collect();
        let filled: Vec<f64> = (0..n)
            .map(|i| {
                if r[i] > 0.0 {
                    return r[i];
                }
                let prev = known
                    .iter()
                    .rev()
                    .find(|&&k| k < i)
                    .or(known.last())
                    .copied()
                    .unwrap_or(i);
                let next = known
                    .iter()
                    .find(|&&k| k > i)
                    .or(known.first())
                    .copied()
                    .unwrap_or(i);
                let span = (next + n - prev) % n;
                let u = if span == 0 {
                    0.5
                } else {
                    ((i + n - prev) % n) as f64 / span as f64
                };
                r[prev] + (r[next] - r[prev]) * u
            })
            .collect();
        filled
    }

    fn outline_points(&self, radii: &[f64]) -> Vec<P2> {
        let n = radii.len();
        let c = self.center;
        (0..n)
            .map(|i| {
                let a = TAU * i as f64 / n as f64;
                [c[0] + radii[i] * a.cos(), c[1] + radii[i] * a.sin()]
            })
            .collect()
    }

    /// The trunk as seen from above, at least visible.
    fn trunk_radius(&self, m_per_mm: f64) -> f64 {
        let r = self
            .branches
            .iter()
            .filter(|b| b.level == 0)
            .map(|b| b.radii[0])
            .fold(0.0, f64::max);
        let r = if r <= 0.0 {
            self.species.trunk / 2.0
        } else {
            r
        };
        r.max(m_per_mm * 0.35).min(self.species.width * 0.12)
    }

    /// The plant drawn in plan.
    pub fn plan(&self, o: &PlanOptions) -> Vec<Shape> {
        let w = (self.radius * 2.0).max(self.species.width * 0.3).max(1e-3);
        let crown_mm = o.crown_mm.max(0.1);
        // Metres per mm of paper.
        let mpm = w / crown_mm;
        let mut style = o.style;
        // Too small on paper for anything but its outline.
        if crown_mm < 5.0 && !matches!(style, PlanStyle::Watercolor) {
            style = PlanStyle::Circle;
        } else if crown_mm < 10.0
            && matches!(
                style,
                PlanStyle::Leaves
                    | PlanStyle::BranchesLeaves
                    | PlanStyle::Stipple
                    | PlanStyle::Foliage
            )
        {
            style = PlanStyle::Outline;
        }
        if style == PlanStyle::Fronds && self.fronds.is_empty() {
            style = PlanStyle::Radial;
        }
        // Palms and rosettes have no branches nor clumps to draw.
        if !self.fronds.is_empty()
            && matches!(
                style,
                PlanStyle::Branches | PlanStyle::Leaves | PlanStyle::BranchesLeaves
            )
        {
            style = PlanStyle::Fronds;
        }
        let mut rng = Rng::new(self.clumps.len() as u64 * 7919 + self.branches.len() as u64);
        let n = 144;
        let raw = self.silhouette(n);
        let soft = smooth_ring(&raw, 3);
        let silhouette = self.outline_points(&soft);
        let perimeter_mm = TAU * (soft.iter().sum::<f64>() / n as f64) / mpm;

        let mut fills: Vec<Vec<P2>> = vec![silhouette.clone()];
        let mut lines: Vec<Shape> = Vec::new();
        let line = |points: Vec<P2>, closed: bool, stroke: Stroke| Shape::Line {
            points,
            closed,
            stroke,
        };
        match style {
            PlanStyle::Circle => {
                let mean = soft.iter().sum::<f64>() / n as f64;
                let c = circle(self.center, mean, 64);
                fills = vec![c.clone()];
                lines.push(line(c, true, Stroke::Outline));
            }
            PlanStyle::Outline => {
                lines.push(line(silhouette.clone(), true, Stroke::Outline));
            }
            PlanStyle::Toothed => {
                let base = smooth_ring(&raw, 6);
                let teeth = ((perimeter_mm / 2.6).round() as usize).clamp(20, 120);
                let depth = (mpm * 0.9).min(0.09 * base.iter().cloned().fold(0.0, f64::max));
                let mut pts = Vec::with_capacity(teeth * 2);
                for i in 0..teeth * 2 {
                    let a =
                        TAU * i as f64 / (teeth * 2) as f64 + rng.signed() * 0.25 / teeth as f64;
                    let k = ((a / TAU * n as f64).round() as usize) % n;
                    let r = if i % 2 == 0 {
                        base[k] + depth * rng.range(-0.2, 0.3)
                    } else {
                        base[k] - depth * rng.range(0.7, 1.2)
                    };
                    pts.push([self.center[0] + r * a.cos(), self.center[1] + r * a.sin()]);
                }
                lines.push(line(pts, true, Stroke::Outline));
            }
            PlanStyle::Cloud => {
                let base = smooth_ring(&raw, 8);
                let lobes = ((perimeter_mm / 9.0).round() as usize).clamp(7, 22);
                let mut cuts: Vec<f64> = (0..lobes)
                    .map(|i| (i as f64 + rng.signed() * 0.3) / lobes as f64 * TAU)
                    .collect();
                cuts.sort_by(f64::total_cmp);
                let mut pts = Vec::new();
                for i in 0..lobes {
                    let a0 = cuts[i];
                    let a1 = if i + 1 < lobes {
                        cuts[i + 1]
                    } else {
                        cuts[0] + TAU
                    };
                    let steps = 10;
                    let swell = rng.range(0.05, 0.1);
                    for j in 0..steps {
                        let u = j as f64 / steps as f64;
                        let a = a0 + (a1 - a0) * u;
                        let k = ((a.rem_euclid(TAU) / TAU * n as f64).round() as usize) % n;
                        let r = base[k] * (1.0 - swell + swell * (PI * u).sin().powf(0.6));
                        pts.push([self.center[0] + r * a.cos(), self.center[1] + r * a.sin()]);
                    }
                }
                fills = vec![pts.clone()];
                lines.push(line(pts, true, Stroke::Outline));
            }
            PlanStyle::Sketch => {
                for pass in 0..2 {
                    let mut wobble = vec![0.0; n];
                    let (h1, h2) = (rng.range(0.0, TAU), rng.range(0.0, TAU));
                    let (k1, k2) = (rng.range(3.0, 6.0).round(), rng.range(7.0, 11.0).round());
                    for (i, wv) in wobble.iter_mut().enumerate() {
                        let a = TAU * i as f64 / n as f64;
                        *wv = 0.035 * (k1 * a + h1).sin() + 0.02 * (k2 * a + h2).sin();
                    }
                    let shift = [rng.signed() * mpm * 0.6, rng.signed() * mpm * 0.6];
                    let radii: Vec<f64> = soft
                        .iter()
                        .zip(&wobble)
                        .map(|(r, w)| r * (1.0 + w))
                        .collect();
                    let pts: Vec<P2> = self
                        .outline_points(&radii)
                        .into_iter()
                        .map(|p| [p[0] + shift[0], p[1] + shift[1]])
                        .collect();
                    lines.push(line(
                        pts,
                        true,
                        if pass == 0 {
                            Stroke::Outline
                        } else {
                            Stroke::Thin
                        },
                    ));
                }
            }
            PlanStyle::Branches => {
                self.branch_lines(&mut lines, crown_mm, mpm, 3);
            }
            PlanStyle::Leaves => {
                self.leaf_marks(&mut lines, mpm, 1.0, &mut rng);
            }
            PlanStyle::BranchesLeaves => {
                self.branch_lines(&mut lines, crown_mm, mpm, 2);
                self.leaf_marks(&mut lines, mpm, 0.55, &mut rng);
            }
            PlanStyle::Foliage => {
                let loops = self.foliage_loops(mpm);
                let outer: Vec<Vec<P2>> = loops.iter().filter(|l| area(l) > 0.0).cloned().collect();
                if !outer.is_empty() {
                    fills = outer;
                }
                for l in loops {
                    lines.push(line(l, true, Stroke::Thin));
                }
            }
            PlanStyle::Stipple => {
                lines.push(line(silhouette.clone(), true, Stroke::Thin));
                let r_mean = soft.iter().sum::<f64>() / n as f64;
                let area_mm = PI * (r_mean / mpm).powi(2);
                let dots = (area_mm * 0.45).min(2600.0) as usize;
                let dot = mpm * 0.18;
                let shade_dir = [0.6, -0.8];
                for _ in 0..dots {
                    let a = rng.range(0.0, TAU);
                    let rr = r_mean * rng.f().sqrt() * 1.05;
                    let p = [self.center[0] + rr * a.cos(), self.center[1] + rr * a.sin()];
                    if !inside(&silhouette, p) {
                        continue;
                    }
                    let along = ((p[0] - self.center[0]) * shade_dir[0]
                        + (p[1] - self.center[1]) * shade_dir[1])
                        / r_mean;
                    let keep = 0.18 + 0.82 * ((along + 1.0) / 2.0).clamp(0.0, 1.0).powf(1.6);
                    if rng.f() > keep {
                        continue;
                    }
                    lines.push(line(
                        vec![
                            [p[0] - dot, p[1] - dot],
                            [p[0] + dot, p[1] - dot],
                            [p[0] + dot, p[1] + dot],
                            [p[0] - dot, p[1] + dot],
                        ],
                        true,
                        Stroke::Thin,
                    ));
                }
            }
            PlanStyle::Radial => {
                let rays = ((perimeter_mm / 2.4).round() as usize).clamp(16, 72);
                for i in 0..rays {
                    let a = TAU * (i as f64 + rng.signed() * 0.35) / rays as f64;
                    let k = ((a.rem_euclid(TAU) / TAU * n as f64).round() as usize) % n;
                    let r_out = soft[k] * rng.range(0.72, 1.0);
                    let r_in = soft[k] * rng.range(0.04, 0.4);
                    let c = self.center;
                    lines.push(line(
                        vec![
                            [c[0] + r_in * a.cos(), c[1] + r_in * a.sin()],
                            [c[0] + r_out * a.cos(), c[1] + r_out * a.sin()],
                        ],
                        false,
                        Stroke::Thin,
                    ));
                }
            }
            PlanStyle::Fronds => {
                fills = Vec::new();
                self.frond_marks(&mut lines, &mut fills, crown_mm);
            }
            PlanStyle::Watercolor => {}
        }

        let mut out = Vec::new();
        let op = o.opacity.clamp(0.02, 1.0);
        let coloured = o.look != Look::Lines || style == PlanStyle::Watercolor;
        if coloured {
            if style == PlanStyle::Watercolor {
                // Washes laid one over another, a little apart, the last
                // on the shaded side.
                out.push(Shape::Fill {
                    points: soften(&silhouette, true),
                    tone: Tone::Leaves,
                    opacity: op * 0.55,
                });
                for k in 0..3 {
                    let s = rng.range(0.62, 0.88);
                    let shift = [rng.signed() * 0.12, rng.signed() * 0.12];
                    let toward = if k == 2 { [0.18, -0.22] } else { shift };
                    let r_mean = soft.iter().sum::<f64>() / n as f64;
                    let wobble = smooth_ring(
                        &raw.iter()
                            .map(|r| r * rng.range(0.85, 1.15))
                            .collect::<Vec<_>>(),
                        10,
                    );
                    let pts: Vec<P2> = self
                        .outline_points(&wobble.iter().map(|r| r * s).collect::<Vec<_>>())
                        .into_iter()
                        .map(|p| [p[0] + toward[0] * r_mean, p[1] + toward[1] * r_mean])
                        .collect();
                    out.push(Shape::Fill {
                        points: soften(&pts, true),
                        tone: Tone::Leaves,
                        opacity: op * if k == 2 { 0.5 } else { 0.32 },
                    });
                }
            } else {
                let fill_op = match o.look {
                    Look::Filled => 1.0,
                    _ => op,
                };
                for f in &fills {
                    out.push(Shape::Fill {
                        points: f.clone(),
                        tone: Tone::Leaves,
                        opacity: fill_op,
                    });
                }
                // The shaded half a little darker, for volume.
                if o.look != Look::Filled && crown_mm > 8.0 && style != PlanStyle::Fronds {
                    let r_mean = soft.iter().sum::<f64>() / n as f64;
                    let shade: Vec<P2> = self
                        .outline_points(&soft.iter().map(|r| r * 0.62).collect::<Vec<_>>())
                        .into_iter()
                        .map(|p| [p[0] + r_mean * 0.22, p[1] - r_mean * 0.26])
                        .filter(|p| inside(&silhouette, *p))
                        .collect();
                    if shade.len() > 8 {
                        out.push(Shape::Fill {
                            points: soften(&shade, true),
                            tone: Tone::Leaves,
                            opacity: fill_op * 0.35,
                        });
                    }
                }
            }
            self.blossoms(&mut out, mpm, crown_mm);
        }
        if o.look != Look::Colour && style != PlanStyle::Watercolor {
            out.extend(lines);
        }
        // The trunk, if it shows.
        if !matches!(style, PlanStyle::Watercolor | PlanStyle::Fronds)
            || self.species.habit == crate::Habit::Palm
        {
            let tr = self.trunk_radius(mpm);
            let foot = self
                .branches
                .iter()
                .find(|b| b.level == 0)
                .map(|b| [b.points[0][0], b.points[0][1]])
                .unwrap_or([0.0, 0.0]);
            let ring = circle(foot, tr, 16);
            if coloured {
                out.push(Shape::Fill {
                    points: ring.clone(),
                    tone: Tone::Bark,
                    opacity: 1.0,
                });
            }
            if o.look != Look::Colour || !coloured {
                out.push(Shape::Line {
                    points: ring,
                    closed: true,
                    stroke: Stroke::Thin,
                });
            }
        }
        out
    }

    /// Branches seen from above: limbs as tapering outlines when wide
    /// enough on paper, the rest as lines.
    fn branch_lines(&self, lines: &mut Vec<Shape>, crown_mm: f64, mpm: f64, max_level: u8) {
        let max_level = if crown_mm > 40.0 {
            max_level
        } else if crown_mm > 18.0 {
            max_level.min(2)
        } else {
            max_level.min(1)
        };
        for b in &self.branches {
            if b.level > max_level || (b.level == 0 && self.species.habit == crate::Habit::Palm) {
                continue;
            }
            let pts: Vec<P2> = b.points.iter().map(|p| [p[0], p[1]]).collect();
            let wide = b.radii[0] * 2.0 / mpm > 0.6 && b.level <= 1;
            if wide {
                if let Some(o) = tapered(b) {
                    lines.push(Shape::Line {
                        points: o,
                        closed: true,
                        stroke: Stroke::Thin,
                    });
                    continue;
                }
            }
            lines.push(Shape::Line {
                points: pts,
                closed: false,
                stroke: Stroke::Thin,
            });
        }
    }

    /// Little marks where the clumps are, kept apart on paper.
    fn leaf_marks(&self, lines: &mut Vec<Shape>, mpm: f64, share: f64, rng: &mut Rng) {
        let mut order: Vec<usize> = (0..self.clumps.len())
            .filter(|&i| self.clumps[i].kind != ClumpKind::Fruit)
            .collect();
        order.sort_by(|&a, &b| self.clumps[b].center[2].total_cmp(&self.clumps[a].center[2]));
        let gap = (mpm * 1.6 / share.max(0.2)).max(self.species.clump * 0.55);
        let mut taken: std::collections::HashMap<(i64, i64), Vec<P2>> = Default::default();
        let size = (self.species.clump * 0.6).clamp(mpm * 1.2, mpm * 2.6);
        for i in order {
            let c = &self.clumps[i];
            let p = [
                c.center[0] + rng.signed() * c.radius * 0.5,
                c.center[1] + rng.signed() * c.radius * 0.5,
            ];
            let key = ((p[0] / gap).floor() as i64, (p[1] / gap).floor() as i64);
            let mut near = false;
            for a in -1..=1 {
                for b in -1..=1 {
                    if let Some(v) = taken.get(&(key.0 + a, key.1 + b)) {
                        if v.iter().any(|q| (q[0] - p[0]).hypot(q[1] - p[1]) < gap) {
                            near = true;
                        }
                    }
                }
            }
            if near {
                continue;
            }
            taken.entry(key).or_default().push(p);
            let rot = rng.range(0.0, TAU);
            let s = size * rng.range(0.75, 1.2);
            let (cs, sn) = (rot.cos(), rot.sin());
            let at = |x: f64, y: f64| [p[0] + x * cs - y * sn, p[1] + x * sn + y * cs];
            match self.species.foliage {
                Foliage::Needles => {
                    for k in 0..3 {
                        let a = k as f64 * PI / 3.0;
                        lines.push(Shape::Line {
                            points: vec![
                                at(-s * a.cos() * 0.6, -s * a.sin() * 0.6),
                                at(s * a.cos() * 0.6, s * a.sin() * 0.6),
                            ],
                            closed: false,
                            stroke: Stroke::Thin,
                        });
                    }
                }
                Foliage::Broom | Foliage::Blades => {
                    lines.push(Shape::Line {
                        points: vec![at(-s * 0.6, 0.0), at(s * 0.6, 0.0)],
                        closed: false,
                        stroke: Stroke::Thin,
                    });
                }
                Foliage::Scales | Foliage::Succulent => {
                    lines.push(Shape::Line {
                        points: circle(p, s * 0.35, 7),
                        closed: true,
                        stroke: Stroke::Thin,
                    });
                }
                Foliage::Fine => {
                    lines.push(Shape::Line {
                        points: vec![
                            at(-s * 0.35, -s * 0.2),
                            at(s * 0.35, -s * 0.2),
                            at(0.0, s * 0.4),
                        ],
                        closed: true,
                        stroke: Stroke::Thin,
                    });
                }
                _ => {
                    // A leaf: pointed at both ends.
                    lines.push(Shape::Line {
                        points: vec![
                            at(-s * 0.55, 0.0),
                            at(-s * 0.15, s * 0.24),
                            at(s * 0.25, s * 0.2),
                            at(s * 0.55, 0.0),
                            at(s * 0.2, -s * 0.2),
                            at(-s * 0.2, -s * 0.22),
                        ],
                        closed: true,
                        stroke: Stroke::Thin,
                    });
                }
            }
        }
    }

    /// The edges of the leaf masses seen from above (and of the gaps
    /// between them), found on a grid round the clumps.
    pub(crate) fn foliage_loops(&self, mpm: f64) -> Vec<Vec<P2>> {
        // Clumps gathered into masses a few to a crown.
        let cell = (self.species.clump * 2.2).max(self.radius / 3.2);
        let merged = crate::mesh::merge(&self.clumps, cell, 0.85);
        let mut rng = Rng::new(merged.len() as u64 + 17);
        let blobs: Vec<(P2, f64, f64, f64)> = merged
            .iter()
            .filter(|c| c.kind != ClumpKind::Fruit)
            .map(|c| {
                (
                    [c.center[0], c.center[1]],
                    c.radius,
                    rng.range(0.0, TAU),
                    rng.range(3.0, 6.0).round(),
                )
            })
            .chain(self.fronds.iter().flat_map(|f| {
                f.rachis
                    .iter()
                    .map(move |p| ([p[0], p[1]], (f.width * 0.6).max(0.05), 0.0, 4.0))
            }))
            .collect();
        if blobs.is_empty() {
            return Vec::new();
        }
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for (p, r, _, _) in &blobs {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k] - r * 1.4);
                hi[k] = hi[k].max(p[k] + r * 1.4);
            }
        }
        let cell = ((hi[0] - lo[0]).max(hi[1] - lo[1]) / 110.0).max(mpm * 0.35);
        let nx = ((hi[0] - lo[0]) / cell).ceil() as usize + 2;
        let ny = ((hi[1] - lo[1]) / cell).ceil() as usize + 2;
        let mut field = vec![0.0_f64; nx * ny];
        for (p, r, ph, lobes) in &blobs {
            let reach = r * 1.3;
            let i0 = (((p[0] - reach - lo[0]) / cell).floor().max(0.0)) as usize;
            let i1 = (((p[0] + reach - lo[0]) / cell).ceil() as usize).min(nx - 1);
            let j0 = (((p[1] - reach - lo[1]) / cell).floor().max(0.0)) as usize;
            let j1 = (((p[1] + reach - lo[1]) / cell).ceil() as usize).min(ny - 1);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let x = lo[0] + i as f64 * cell - p[0];
                    let y = lo[1] + j as f64 * cell - p[1];
                    // A lumpy disc: 1 in its middle, 0 at its edge.
                    let a = y.atan2(x);
                    let edge = r * (1.0 + 0.16 * (lobes * a + ph).sin());
                    let f = 1.0 - (x * x + y * y).sqrt() / edge;
                    let v = &mut field[j * nx + i];
                    *v = v.max(f);
                }
            }
        }
        // The edge of the whole, and each mass inside it.
        let mut loops = contours(&field, nx, ny, 0.0);
        loops.extend(contours(&field, nx, ny, 0.42));
        loops
            .into_iter()
            .filter(|l| l.len() >= 6)
            .map(|l| {
                let pts: Vec<P2> = l
                    .into_iter()
                    .map(|[x, y]| [lo[0] + x * cell, lo[1] + y * cell])
                    .collect();
                soften(&pts, true)
            })
            .filter(|l| area(l).abs() > (mpm * 1.2).powi(2))
            .collect()
    }

    /// Fronds seen from above: the outline of each leaf, its midrib and,
    /// big enough on paper, its leaflets or pleats.
    fn frond_marks(&self, lines: &mut Vec<Shape>, fills: &mut Vec<Vec<P2>>, crown_mm: f64) {
        for f in &self.fronds {
            let m = f.rachis.len();
            if m < 2 {
                continue;
            }
            let pts: Vec<P2> = f.rachis.iter().map(|p| [p[0], p[1]]).collect();
            let side = [f.side[0], f.side[1]];
            let sl = side[0].hypot(side[1]).max(1e-9);
            let side = [side[0] / sl, side[1] / sl];
            match f.kind {
                FrondKind::Pinnate | FrondKind::Blade | FrondKind::Sword => {
                    let mut left = Vec::new();
                    let mut right = Vec::new();
                    for (i, p) in pts.iter().enumerate() {
                        let s = i as f64 / (m - 1) as f64;
                        let w = match f.kind {
                            FrondKind::Pinnate => {
                                f.width * (PI * s.powf(0.75)).sin().max(0.0).powf(0.6)
                            }
                            _ => f.width * (1.0 - s).powf(0.8),
                        };
                        left.push([p[0] + side[0] * w, p[1] + side[1] * w]);
                        right.push([p[0] - side[0] * w, p[1] - side[1] * w]);
                    }
                    let mut outline = left.clone();
                    outline.extend(right.iter().rev());
                    if f.kind == FrondKind::Pinnate && crown_mm > 25.0 {
                        // Leaflets as a zigzag along each side.
                        let mut zig = Vec::new();
                        for i in 0..m {
                            zig.push(pts[i]);
                            zig.push(left[i]);
                        }
                        lines.push(Shape::Line {
                            points: zig,
                            closed: false,
                            stroke: Stroke::Thin,
                        });
                        let mut zag = Vec::new();
                        for i in 0..m {
                            zag.push(pts[i]);
                            zag.push(right[i]);
                        }
                        lines.push(Shape::Line {
                            points: zag,
                            closed: false,
                            stroke: Stroke::Thin,
                        });
                    } else {
                        lines.push(Shape::Line {
                            points: outline.clone(),
                            closed: true,
                            stroke: Stroke::Thin,
                        });
                    }
                    if crown_mm > 12.0 {
                        lines.push(Shape::Line {
                            points: pts.clone(),
                            closed: false,
                            stroke: Stroke::Thin,
                        });
                    }
                    fills.push(outline);
                }
                FrondKind::Fan => {
                    lines.push(Shape::Line {
                        points: pts.clone(),
                        closed: false,
                        stroke: Stroke::Thin,
                    });
                    let c = pts[m - 1];
                    let d = [pts[m - 1][0] - pts[m - 2][0], pts[m - 1][1] - pts[m - 2][1]];
                    let dl = d[0].hypot(d[1]).max(1e-9);
                    let d = [d[0] / dl, d[1] / dl];
                    let mut wedge = vec![c];
                    let segs = 12;
                    for k in 0..=segs {
                        let a = (-80.0 + 160.0 * k as f64 / segs as f64).to_radians();
                        let r = f.width * if k % 2 == 0 { 1.0 } else { 0.88 };
                        let e = [
                            c[0] + r * (a.cos() * d[0] - a.sin() * d[1]),
                            c[1] + r * (a.cos() * d[1] + a.sin() * d[0]),
                        ];
                        if crown_mm > 20.0 && k % 2 == 0 {
                            lines.push(Shape::Line {
                                points: vec![c, e],
                                closed: false,
                                stroke: Stroke::Thin,
                            });
                        }
                        wedge.push(e);
                    }
                    lines.push(Shape::Line {
                        points: wedge.clone(),
                        closed: true,
                        stroke: Stroke::Thin,
                    });
                    fills.push(wedge);
                }
            }
        }
    }

    /// Flowers and fruit as spots of their colour.
    fn blossoms(&self, out: &mut Vec<Shape>, mpm: f64, crown_mm: f64) {
        if crown_mm < 8.0 {
            return;
        }
        let mut seen: std::collections::HashSet<(i64, i64)> = Default::default();
        let gap = mpm * 2.6;
        for c in &self.clumps {
            let tone = match c.kind {
                ClumpKind::Flowers => Tone::Flowers,
                ClumpKind::Fruit => Tone::Fruit,
                ClumpKind::Leaves => continue,
            };
            let key = (
                (c.center[0] / gap).floor() as i64,
                (c.center[1] / gap).floor() as i64,
            );
            if !seen.insert(key) {
                continue;
            }
            let r = match c.kind {
                ClumpKind::Fruit => c.radius.max(mpm * 0.45),
                _ => (c.radius * 0.45).max(mpm * 0.5),
            };
            out.push(Shape::Fill {
                points: circle([c.center[0], c.center[1]], r, 8),
                tone,
                opacity: 0.85,
            });
        }
    }
}

/// The outline of a branch seen from above, tapering as it goes.
fn tapered(b: &Branch) -> Option<Vec<P2>> {
    let n = b.points.len();
    if n < 2 {
        return None;
    }
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    for i in 0..n {
        let a = b.points[i.saturating_sub(1)];
        let c = b.points[(i + 1).min(n - 1)];
        let d = [c[0] - a[0], c[1] - a[1]];
        let l = d[0].hypot(d[1]);
        if l < 1e-9 {
            return None;
        }
        let side = [-d[1] / l, d[0] / l];
        let r = b.radii[i];
        let p = b.points[i];
        left.push([p[0] + side[0] * r, p[1] + side[1] * r]);
        right.push([p[0] - side[0] * r, p[1] - side[1] * r]);
    }
    left.extend(right.into_iter().rev());
    Some(left)
}

/// Lines where a field crosses a value (marching squares), joined into
/// closed loops, in grid units.
fn contours(field: &[f64], nx: usize, ny: usize, level: f64) -> Vec<Vec<P2>> {
    use std::collections::HashMap;
    // Edges: (i, j, 0) from (i, j) to (i + 1, j); (i, j, 1) to (i, j + 1).
    type E = (usize, usize, u8);
    let v = |i: usize, j: usize| field[j * nx + i] - level;
    let point = |e: E| -> P2 {
        let (i, j, k) = e;
        let (a, b) = if k == 0 {
            (v(i, j), v(i + 1, j))
        } else {
            (v(i, j), v(i, j + 1))
        };
        let t = if (a - b).abs() < 1e-12 {
            0.5
        } else {
            a / (a - b)
        };
        if k == 0 {
            [i as f64 + t, j as f64]
        } else {
            [i as f64, j as f64 + t]
        }
    };
    let mut next: HashMap<E, E> = HashMap::new();
    for j in 0..ny - 1 {
        for i in 0..nx - 1 {
            let c = [v(i, j), v(i + 1, j), v(i + 1, j + 1), v(i, j + 1)];
            let mut case = 0;
            for (k, x) in c.iter().enumerate() {
                if *x > 0.0 {
                    case |= 1 << k;
                }
            }
            let bottom = (i, j, 0);
            let right = (i + 1, j, 1);
            let top = (i, j + 1, 0);
            let left = (i, j, 1);
            // Segments go with the inside on their left.
            let segs: &[(E, E)] = match case {
                1 => &[(left, bottom)],
                2 => &[(bottom, right)],
                3 => &[(left, right)],
                4 => &[(right, top)],
                5 => {
                    if (c.iter().sum::<f64>()) > 0.0 {
                        &[(left, top), (right, bottom)]
                    } else {
                        &[(left, bottom), (right, top)]
                    }
                }
                6 => &[(bottom, top)],
                7 => &[(left, top)],
                8 => &[(top, left)],
                9 => &[(top, bottom)],
                10 => {
                    if (c.iter().sum::<f64>()) > 0.0 {
                        &[(bottom, left), (top, right)]
                    } else {
                        &[(bottom, right), (top, left)]
                    }
                }
                11 => &[(top, right)],
                12 => &[(right, left)],
                13 => &[(right, bottom)],
                14 => &[(bottom, left)],
                _ => &[],
            };
            for &(a, b) in segs {
                // Inside on the left: from `b` to `a` reversed as needed.
                next.insert(b, a);
            }
        }
    }
    let mut loops = Vec::new();
    let mut used: std::collections::HashSet<E> = Default::default();
    let mut keys: Vec<E> = next.keys().copied().collect();
    keys.sort();
    for start in keys {
        if used.contains(&start) {
            continue;
        }
        let mut l = Vec::new();
        let mut e = start;
        loop {
            if !used.insert(e) {
                break;
            }
            l.push(point(e));
            match next.get(&e) {
                Some(n) => e = *n,
                None => break,
            }
        }
        if l.len() >= 3 {
            loops.push(l);
        }
    }
    loops
}
