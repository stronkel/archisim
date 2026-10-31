//! The bars of the reinforcement (E11): laid by rule from what the
//! designer wrote for each element (`archisim_core::Reinforcement`) inside
//! its concrete, with its cover — a column's bars round its section and
//! its ties (closer at the ends), a beam's bars at the bottom and the top
//! and its stirrups, the meshes of footings (with their bent ends) and
//! slabs (cut where the slab ends and at its holes) — as polylines with a
//! diameter, for the 3D, the plan, the schedule of bars and the IFC.

use archisim_core::{Model, Point2, Profile, RebarKind, Reinforcement, Spaced};

use crate::solid::{Solid, SolidOwner, P3};

/// What a bar does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BarRole {
    /// Longitudinal bars of columns and beams.
    Main,
    /// Ties and stirrups.
    Tie,
    /// Bars of a mesh (footings, slabs).
    Mesh,
}

/// Bars of one kind in an element: «4Ø16», «Ø8 c/15».
#[derive(Debug, Clone, PartialEq)]
pub struct BarGroup {
    /// How it is written on a drawing.
    pub label: String,
    /// Where it goes: «longitudinales», «cercos», «inferior x»…
    pub place: String,
    pub role: BarRole,
    pub diameter: f64,
    /// The bars, each as the line of its axis (a closed tie repeats its
    /// first point at the end).
    pub bars: Vec<Vec<P3>>,
}

impl BarGroup {
    /// The length of all its bars together, metres.
    pub fn length(&self) -> f64 {
        self.bars.iter().map(|b| polyline_length(b)).sum()
    }

    /// Its weight, kg.
    pub fn weight(&self) -> f64 {
        self.length() * archisim_core::bar_weight(self.diameter)
    }
}

fn polyline_length(b: &[P3]) -> f64 {
    b.windows(2)
        .map(|w| {
            let d = [w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]];
            (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
        })
        .sum()
}

/// Positions along a length `len` at spacing `s`, the first and last at
/// most `s` / 2 from the ends (as many as fit).
fn spaced_along(len: f64, s: f64) -> Vec<f64> {
    if len <= 0.0 || s <= 0.0 {
        return Vec::new();
    }
    let n = (len / s).ceil().max(1.0) as usize;
    let step = len / n as f64;
    (0..n).map(|i| step * (i as f64 + 0.5)).collect()
}

/// `n` positions spread evenly across `width` (centres), the outer ones on
/// its edges.
fn across(width: f64, n: u32) -> Vec<f64> {
    match n {
        0 => Vec::new(),
        1 => vec![width / 2.0],
        _ => (0..n).map(|i| width * i as f64 / (n - 1) as f64).collect(),
    }
}

/// The bars of an element with reinforcement, in groups.
pub fn element_bars(model: &Model, id: u64) -> Vec<BarGroup> {
    let Some(r) = model.reinforcement(id).copied() else {
        return Vec::new();
    };
    match r {
        Reinforcement::Column {
            cover,
            bars,
            ties,
            dense,
        } => column_bars(model, id, cover, bars, ties, dense),
        Reinforcement::Beam {
            cover,
            bottom,
            top,
            stirrups,
        } => beam_bars(model, id, cover, bottom, top, stirrups),
        Reinforcement::Footing { cover, bottom, top } => {
            footing_bars(model, id, cover, bottom, top)
        }
        Reinforcement::Slab { cover, bottom, top } => slab_bars(model, id, cover, bottom, top),
    }
}

fn column_bars(
    model: &Model,
    id: u64,
    cover: f64,
    bars: archisim_core::Counted,
    ties: Spaced,
    dense: Option<(f64, f64)>,
) -> Vec<BarGroup> {
    let Some(c) = model.column(archisim_core::ColumnId(id)) else {
        return Vec::new();
    };
    let Some(t) = model.member_type(c.member_type) else {
        return Vec::new();
    };
    let base = model.level(c.level).map_or(0.0, |l| l.elevation) + c.base_offset;
    let top = base + c.height;
    let (rc, rs) = (c.rotation.cos(), c.rotation.sin());
    let place = |x: f64, y: f64| {
        Point2::new(
            c.position.x + x * rc - y * rs,
            c.position.y + x * rs + y * rc,
        )
    };
    // The line of the ties and that of the bars' centres.
    let tie_in = cover + ties.diameter / 2.0;
    let bar_in = cover + ties.diameter + bars.diameter / 2.0;
    let (main, loop_): (Vec<Point2>, Vec<Point2>) = match t.profile {
        Profile::Circle { diameter } => {
            let r_bar = diameter / 2.0 - bar_in;
            let r_tie = diameter / 2.0 - tie_in;
            let n = bars.count.max(1);
            let main = (0..n)
                .map(|i| {
                    let a = std::f64::consts::TAU * i as f64 / n as f64;
                    place(r_bar * a.cos(), r_bar * a.sin())
                })
                .collect();
            let tie = (0..=16)
                .map(|i| {
                    let a = std::f64::consts::TAU * i as f64 / 16.0;
                    place(r_tie * a.cos(), r_tie * a.sin())
                })
                .collect();
            (main, tie)
        }
        Profile::Rectangle { width, depth } => {
            let (hx, hy) = (width / 2.0 - bar_in, depth / 2.0 - bar_in);
            let (tx, ty) = (width / 2.0 - tie_in, depth / 2.0 - tie_in);
            let tie = vec![
                place(-tx, -ty),
                place(tx, -ty),
                place(tx, ty),
                place(-tx, ty),
                place(-tx, -ty),
            ];
            (
                rect_bars(hx, hy, bars.count)
                    .into_iter()
                    .map(|(x, y)| place(x, y))
                    .collect(),
                tie,
            )
        }
        _ => return Vec::new(),
    };
    let main_group = BarGroup {
        label: bars.label(),
        place: "longitudinales".into(),
        role: BarRole::Main,
        diameter: bars.diameter,
        bars: main
            .iter()
            .map(|p| vec![[p.x, p.y, base + cover], [p.x, p.y, top - cover]])
            .collect(),
    };
    // The ties: closer along `dense` from both ends.
    let mut zs = Vec::new();
    let h = top - base;
    match dense.filter(|(s, l)| *s > 0.0 && *l > 0.0 && 2.0 * l < h) {
        Some((ds, dl)) => {
            zs.extend(spaced_along(dl, ds).into_iter().map(|z| base + z));
            zs.extend(
                spaced_along(h - 2.0 * dl, ties.spacing)
                    .into_iter()
                    .map(|z| base + dl + z),
            );
            zs.extend(spaced_along(dl, ds).into_iter().map(|z| top - dl + z));
        }
        None => zs.extend(spaced_along(h, ties.spacing).into_iter().map(|z| base + z)),
    }
    let tie_group = BarGroup {
        label: format!("cercos {}", ties.label()),
        place: "cercos".into(),
        role: BarRole::Tie,
        diameter: ties.diameter,
        bars: zs
            .into_iter()
            .map(|z| loop_.iter().map(|p| [p.x, p.y, z]).collect())
            .collect(),
    };
    vec![main_group, tie_group]
}

/// `n` bars round a rectangle of half sides `hx`, `hy`: the corners first,
/// the rest on the sides, the longer ones first.
fn rect_bars(hx: f64, hy: f64, n: u32) -> Vec<(f64, f64)> {
    let mut out = vec![(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)];
    if n < 4 {
        out.truncate(n as usize);
        return out;
    }
    // Bars between the corners of each side: x sides (bottom, top) and y
    // sides (right, left), shared out by length.
    let extra = n - 4;
    let (lx, ly) = (2.0 * hx, 2.0 * hy);
    let mut per = [0u32; 4];
    for k in 0..extra {
        // The side whose bars are farthest apart takes the next one.
        let gaps: Vec<f64> = (0..4)
            .map(|i| if i % 2 == 0 { lx } else { ly } / (per[i] + 1) as f64)
            .collect();
        let i = (0..4)
            .max_by(|a, b| gaps[*a].total_cmp(&gaps[*b]).then(b.cmp(a)))
            .unwrap_or(k as usize % 4);
        per[i] += 1;
    }
    for (i, &m) in per.iter().enumerate() {
        for j in 1..=m {
            let f = j as f64 / (m + 1) as f64;
            out.push(match i {
                0 => (-hx + lx * f, -hy),
                1 => (hx, -hy + ly * f),
                2 => (hx - lx * f, hy),
                _ => (-hx, hy - ly * f),
            });
        }
    }
    out
}

fn beam_bars(
    model: &Model,
    id: u64,
    cover: f64,
    bottom: archisim_core::Counted,
    top: archisim_core::Counted,
    stirrups: Spaced,
) -> Vec<BarGroup> {
    let Some(b) = model.beam(archisim_core::BeamId(id)) else {
        return Vec::new();
    };
    let Some(t) = model.member_type(b.member_type) else {
        return Vec::new();
    };
    let Some(u) = (b.end - b.start).normalized() else {
        return Vec::new();
    };
    let n = u.perp_left();
    let z_top = model.level(b.level).map_or(0.0, |l| l.elevation) + b.top_offset;
    let (w, d) = t.profile.size();
    let len = b.length();
    let at = |s: f64, x: f64, z: f64| {
        let p = b.start + u * s + n * x;
        [p.x, p.y, z]
    };
    let s_in = cover + stirrups.diameter;
    let row = |c: archisim_core::Counted, z: f64| -> Vec<Vec<P3>> {
        let inner = w - 2.0 * (s_in + c.diameter / 2.0);
        across(inner, c.count)
            .into_iter()
            .map(|x| {
                let x = x - inner / 2.0;
                vec![at(cover, x, z), at(len - cover, x, z)]
            })
            .collect()
    };
    let z_bottom = z_top - d + s_in + bottom.diameter / 2.0;
    let z_up = z_top - s_in - top.diameter / 2.0;
    let (sx, sz0, sz1) = (
        w / 2.0 - cover - stirrups.diameter / 2.0,
        z_top - d + cover + stirrups.diameter / 2.0,
        z_top - cover - stirrups.diameter / 2.0,
    );
    let stirrup_bars = spaced_along(len, stirrups.spacing)
        .into_iter()
        .map(|s| {
            vec![
                at(s, -sx, sz0),
                at(s, sx, sz0),
                at(s, sx, sz1),
                at(s, -sx, sz1),
                at(s, -sx, sz0),
            ]
        })
        .collect();
    vec![
        BarGroup {
            label: bottom.label(),
            place: "inferior".into(),
            role: BarRole::Main,
            diameter: bottom.diameter,
            bars: row(bottom, z_bottom),
        },
        BarGroup {
            label: top.label(),
            place: "superior".into(),
            role: BarRole::Main,
            diameter: top.diameter,
            bars: row(top, z_up),
        },
        BarGroup {
            label: format!("estribos {}", stirrups.label()),
            place: "estribos".into(),
            role: BarRole::Tie,
            diameter: stirrups.diameter,
            bars: stirrup_bars,
        },
    ]
}

/// A mesh of two layers on a rectangle: `x` along `ux` for `lx` metres,
/// `y` along `uy` for `ly`, from `origin`, its first layer at `z`, the
/// second `step` above (below, for a top mesh). The bars bend `legs` up
/// (down) at their ends.
#[allow(clippy::too_many_arguments)]
fn mesh_on_rect(
    origin: Point2,
    ux: Point2,
    uy: Point2,
    lx: f64,
    ly: f64,
    cover: f64,
    mesh: [Spaced; 2],
    z: f64,
    up: f64,
    legs: f64,
    place: &str,
) -> Vec<BarGroup> {
    let mut out = Vec::new();
    let mut z = z + up * mesh[0].diameter / 2.0;
    for (k, (s, (along, other, l_along, l_other))) in mesh
        .iter()
        .zip([(ux, uy, lx, ly), (uy, ux, ly, lx)])
        .enumerate()
    {
        if k == 1 {
            z += up * (mesh[0].diameter + mesh[1].diameter) / 2.0;
        }
        let bars = spaced_along(l_other - 2.0 * cover, s.spacing)
            .into_iter()
            .map(|t| {
                let a = origin + other * (cover + t) + along * cover;
                let b = a + along * (l_along - 2.0 * cover);
                let mut bar = vec![[a.x, a.y, z], [b.x, b.y, z]];
                if legs > 0.0 {
                    bar.insert(0, [a.x, a.y, z + up * legs]);
                    bar.push([b.x, b.y, z + up * legs]);
                }
                bar
            })
            .collect();
        out.push(BarGroup {
            label: s.label(),
            place: format!("{place} {}", if k == 0 { "x" } else { "y" }),
            role: BarRole::Mesh,
            diameter: s.diameter,
            bars,
        });
    }
    out
}

fn footing_bars(
    model: &Model,
    id: u64,
    cover: f64,
    bottom: [Spaced; 2],
    top: Option<[Spaced; 2]>,
) -> Vec<BarGroup> {
    let Some(f) = model.footing(archisim_core::FootingId(id)) else {
        return Vec::new();
    };
    let o = f.outline();
    if o.len() != 4 {
        return Vec::new();
    }
    let (Some(ux), Some(uy)) = ((o[1] - o[0]).normalized(), (o[3] - o[0]).normalized()) else {
        return Vec::new();
    };
    let (lx, ly) = (o[0].distance(o[1]), o[0].distance(o[3]));
    let top_z = model.level(f.level).map_or(0.0, |l| l.elevation) + f.top_offset;
    let bottom_z = top_z - f.depth;
    // The bent ends of the bottom mesh: up to half the depth.
    let legs = ((f.depth - 2.0 * cover) / 2.0).clamp(0.0, 0.25);
    let mut out = mesh_on_rect(
        o[0],
        ux,
        uy,
        lx,
        ly,
        cover,
        bottom,
        bottom_z + cover,
        1.0,
        legs,
        "inferior",
    );
    if let Some(t) = top {
        out.extend(mesh_on_rect(
            o[0],
            ux,
            uy,
            lx,
            ly,
            cover,
            t,
            top_z - cover,
            -1.0,
            0.0,
            "superior",
        ));
    }
    out
}

fn slab_bars(
    model: &Model,
    id: u64,
    cover: f64,
    bottom: [Spaced; 2],
    top: Option<[Spaced; 2]>,
) -> Vec<BarGroup> {
    let Some(s) = model.slab(archisim_core::SlabId(id)) else {
        return Vec::new();
    };
    let Some(t) = model.slab_type(s.slab_type) else {
        return Vec::new();
    };
    if s.outline.len() < 3 || s.slope.is_some() {
        return Vec::new();
    }
    let Some(ux) = (s.outline[1] - s.outline[0]).normalized() else {
        return Vec::new();
    };
    let uy = ux.perp_left();
    // The bars go in its structural layer (under the floor finishes), or
    // in the whole of it if it has none.
    let slab_top = model.level(s.level).map_or(0.0, |l| l.elevation) + s.top_offset;
    let (mut top_z, mut bottom_z) = (slab_top, slab_top - t.thickness());
    let mut at = slab_top;
    for l in &t.layers {
        if l.function == archisim_core::LayerFunction::Structure {
            (top_z, bottom_z) = (at, at - l.thickness);
            break;
        }
        at -= l.thickness;
    }
    // The slab in the axes of its first edge.
    let local: Vec<Point2> = s
        .outline
        .iter()
        .map(|p| Point2::new((*p - s.outline[0]).dot(ux), (*p - s.outline[0]).dot(uy)))
        .collect();
    let (lo, hi) = local.iter().fold(
        (
            Point2::new(f64::MAX, f64::MAX),
            Point2::new(f64::MIN, f64::MIN),
        ),
        |(lo, hi), p| {
            (
                Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    );
    let world = |x: f64, y: f64| s.outline[0] + ux * x + uy * y;
    let mut out = Vec::new();
    let layers: Vec<([Spaced; 2], f64, f64, &str)> =
        std::iter::once((bottom, bottom_z + cover, 1.0, "inferior"))
            .chain(top.map(|t| (t, top_z - cover, -1.0, "superior")))
            .collect();
    for (mesh, z0, up, name) in layers {
        let mut z = z0 + up * mesh[0].diameter / 2.0;
        for (k, sp) in mesh.iter().enumerate() {
            if k == 1 {
                z += up * (mesh[0].diameter + mesh[1].diameter) / 2.0;
            }
            let mut bars = Vec::new();
            // Lines along x (k = 0) spaced across y, or along y.
            let (from, to) = if k == 0 { (lo.y, hi.y) } else { (lo.x, hi.x) };
            for t in spaced_along(to - from - 2.0 * cover, sp.spacing) {
                let c = from + cover + t;
                let (a, b) = if k == 0 {
                    (world(lo.x - 1.0, c), world(hi.x + 1.0, c))
                } else {
                    (world(c, lo.y - 1.0), world(c, hi.y + 1.0))
                };
                for (f0, f1) in crate::facade::segment_inside(a, b, &s.outline, &s.holes) {
                    let (p, q) = (a + (b - a) * f0, a + (b - a) * f1);
                    let Some(dir) = (q - p).normalized() else {
                        continue;
                    };
                    if p.distance(q) <= 2.0 * cover {
                        continue;
                    }
                    let (p, q) = (p + dir * cover, q - dir * cover);
                    bars.push(vec![[p.x, p.y, z], [q.x, q.y, z]]);
                }
            }
            out.push(BarGroup {
                label: sp.label(),
                place: format!("{name} {}", if k == 0 { "x" } else { "y" }),
                role: BarRole::Mesh,
                diameter: sp.diameter,
                bars,
            });
        }
    }
    out
}

/// The owner of an element's solids.
pub fn owner_of(model: &Model, id: u64) -> Option<SolidOwner> {
    Some(match model.rebar_kind(id)? {
        RebarKind::Column => SolidOwner::Column(archisim_core::ColumnId(id)),
        RebarKind::Beam => SolidOwner::Beam(archisim_core::BeamId(id)),
        RebarKind::Footing => SolidOwner::Footing(archisim_core::FootingId(id)),
        RebarKind::Slab => SolidOwner::Slab(archisim_core::SlabId(id)),
    })
}

/// Sides of the bars in 3D.
const SIDES: usize = 6;

/// The bars of an element as solids: each bar a tube of its diameter.
pub fn rebar_solids(model: &Model, id: u64) -> Vec<Solid> {
    let Some(owner) = owner_of(model, id) else {
        return Vec::new();
    };
    let Some(material) = model.rebar_material() else {
        return Vec::new();
    };
    let mut triangles = Vec::new();
    for g in element_bars(model, id) {
        for bar in &g.bars {
            tube(bar, g.diameter / 2.0, &mut triangles);
        }
    }
    if triangles.is_empty() {
        return Vec::new();
    }
    vec![Solid {
        owner,
        material,
        triangles,
    }]
}

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: P3) -> Option<P3> {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    (l > 1e-12).then(|| [a[0] / l, a[1] / l, a[2] / l])
}

/// A tube along a polyline, segment by segment, closed at its ends.
pub fn tube(points: &[P3], r: f64, out: &mut Vec<[P3; 3]>) {
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        let Some(d) = unit(sub(b, a)) else {
            continue;
        };
        let helper = if d[2].abs() < 0.9 {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let Some(e1) = unit(cross(d, helper)) else {
            continue;
        };
        let e2 = cross(d, e1);
        let ring = |c: P3| -> Vec<P3> {
            (0..SIDES)
                .map(|i| {
                    let t = std::f64::consts::TAU * i as f64 / SIDES as f64;
                    let (cs, sn) = (t.cos() * r, t.sin() * r);
                    [
                        c[0] + e1[0] * cs + e2[0] * sn,
                        c[1] + e1[1] * cs + e2[1] * sn,
                        c[2] + e1[2] * cs + e2[2] * sn,
                    ]
                })
                .collect()
        };
        let (ra, rb) = (ring(a), ring(b));
        for i in 0..SIDES {
            let j = (i + 1) % SIDES;
            out.push([ra[i], ra[j], rb[j]]);
            out.push([ra[i], rb[j], rb[i]]);
        }
        for i in 1..SIDES - 1 {
            out.push([ra[0], ra[i + 1], ra[i]]);
            out.push([rb[0], rb[i], rb[i + 1]]);
        }
    }
}

/// A bar laid flat for the bar schedule: its axis in its own plane, the
/// first stretch along +x and the first bend towards +y, so equal bars
/// give equal shapes wherever they are in the element. Stretches shorter
/// than a millimetre are left out.
pub fn bar_shape(bar: &[P3]) -> Vec<[f64; 2]> {
    let mut pts: Vec<P3> = Vec::new();
    for p in bar {
        if pts.last().is_none_or(|q| {
            let d = sub(*p, *q);
            (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() > 1e-3
        }) {
            pts.push(*p);
        }
    }
    if pts.len() < 2 {
        return Vec::new();
    }
    let Some(u) = unit(sub(pts[1], pts[0])) else {
        return Vec::new();
    };
    // The plane: the first stretch not along the first one.
    let mut v = None;
    for w in pts.windows(2).skip(1) {
        if let Some(d) = unit(sub(w[1], w[0])) {
            if let Some(n) = unit(cross(u, d)) {
                v = Some(cross(n, u));
                break;
            }
        }
    }
    let v = v.unwrap_or([0.0, 0.0, 0.0]);
    let o = pts[0];
    pts.iter()
        .map(|p| {
            let d = sub(*p, o);
            [
                d[0] * u[0] + d[1] * u[1] + d[2] * u[2],
                d[0] * v[0] + d[1] * v[1] + d[2] * v[2],
            ]
        })
        .collect()
}

/// The lengths of the stretches of a flat bar.
pub fn shape_stretches(shape: &[[f64; 2]]) -> Vec<f64> {
    shape
        .windows(2)
        .map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bars_round_a_rectangle() {
        assert_eq!(rect_bars(0.1, 0.1, 4).len(), 4);
        let six = rect_bars(0.2, 0.1, 6);
        assert_eq!(six.len(), 6);
        // The two more go on the long sides (x), in their middle.
        assert!(six[4..].iter().all(|(x, _)| x.abs() < 1e-9));
        assert_eq!(rect_bars(0.1, 0.1, 8).len(), 8);
        assert_eq!(spaced_along(1.0, 0.25).len(), 4);
    }

    #[test]
    fn bars_lie_flat_whichever_way_they_go() {
        // A footing bar with its two bent ends, standing in the y–z plane.
        let bar = vec![
            [1.0, 0.0, 0.5],
            [1.0, 0.0, 0.0],
            [1.0, 1.2, 0.0],
            [1.0, 1.2, 0.5],
        ];
        let s = bar_shape(&bar);
        let l = shape_stretches(&s);
        assert_eq!(l.len(), 3);
        assert!((l[0] - 0.5).abs() < 1e-9 && (l[1] - 1.2).abs() < 1e-9);
        // First along x, then up y.
        assert!((s[1][0] - 0.5).abs() < 1e-9 && s[1][1].abs() < 1e-9);
        assert!(s[2][1] > 1.1);
        // A straight bar, and repeated points dropped.
        let s = bar_shape(&[[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 3.0]]);
        assert_eq!(s.len(), 2);
        assert!((s[1][0] - 3.0).abs() < 1e-9);
    }
}
