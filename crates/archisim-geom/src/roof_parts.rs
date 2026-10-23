//! The solids of what a roof carries (`docs/MAQUETA.md` §5): dormers and
//! roof windows, tiles with their courses and covers, ridge tiles over
//! ridges and hips, gutters and downpipes.
//!
//! All of it is light: a dormer is a few dozen triangles, the tiles of a
//! house a few thousand (a plate per course and a rib per cover), so the
//! 3D view stays fluid on an old laptop.

use archisim_core::{
    Dormer, DormerKind, DormerShape, Model, Point2, Roof, RoofId, TileKind, Vec2, DORMER_FRAME,
    DORMER_WALL,
};

use crate::junctions::{self, Shape};
use crate::solid::{
    layered_plate_pub as layered_plate, prism, sweep_segment, vertical_prism_pub as vertical_prism,
    Solid, SolidOwner, P3,
};

/// What a piece of a dormer is (for the IFC: walls, roof, window).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DormerPart {
    /// Front and cheeks.
    Walls,
    /// Its own roof.
    Roof,
    /// The window's frame.
    Frame,
    Glass,
}

/// Vertical thickness of a roof (its layers, measured square to the
/// slope).
fn roof_vertical(model: &Model, roof: &Roof, slope: f64) -> f64 {
    let t = model
        .slab_type(roof.roof_type)
        .map_or(0.25, |t| t.thickness());
    t * (1.0 + slope * slope).sqrt()
}

fn level_z(model: &Model, roof: &Roof) -> f64 {
    model.level(roof.level).map_or(0.0, |l| l.elevation)
}

/// The part of a polygon (`x` across, `y` up) below or above `at`.
fn clip_y(poly: &[Point2], at: f64, above: bool) -> Vec<Point2> {
    let f = |p: Point2| if above { p.y - at } else { at - p.y };
    let n = poly.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (fa, fb) = (f(a), f(b));
        if fa >= -1e-12 {
            out.push(a);
        }
        if (fa < -1e-12 && fb > 1e-12) || (fa > 1e-12 && fb < -1e-12) {
            let t = fa / (fa - fb);
            out.push(a + (b - a) * t);
        }
    }
    out
}

fn clip_x(poly: &[Point2], at: f64, right: bool) -> Vec<Point2> {
    let flip: Vec<Point2> = poly.iter().map(|p| Point2::new(p.y, p.x)).collect();
    clip_y(&flip, at, right)
        .into_iter()
        .map(|p| Point2::new(p.y, p.x))
        .collect()
}

fn ok(poly: &[Point2]) -> bool {
    poly.len() >= 3 && archisim_core::signed_area(poly).abs() > 1e-9
}

/// The pieces of a dormer, each with what it is.
pub fn dormer_parts(model: &Model, id: RoofId, index: usize) -> Vec<(DormerPart, Solid)> {
    let Some(roof) = model.roof(id) else {
        return Vec::new();
    };
    let Some(d) = roof.dormers.get(index) else {
        return Vec::new();
    };
    let Some(shape) = roof.dormer_shape(d) else {
        return Vec::new();
    };
    let owner = SolidOwner::Dormer(id, index as u16);
    let lz = level_z(model, roof);
    let f = shape.frame;
    let below = roof_vertical(model, roof, f.slope);
    let mut out = Vec::new();
    if d.kind == DormerKind::Skylight {
        skylight(model, roof, d, &shape, owner, lz, below, &mut out);
        return out;
    }
    let lift = |poly: Vec<Point2>| -> Vec<Point2> {
        poly.into_iter()
            .map(|p| Point2::new(p.x, p.y + lz))
            .collect()
    };
    let t = DORMER_WALL;
    // The front, with its window taken out.
    let front = lift(shape.front_profile(below, d.width));
    let window = shape
        .window(d)
        .map(|[x0, z0, x1, z1]| [x0, z0 + lz, x1, z1 + lz]);
    let pieces: Vec<Vec<Point2>> = match window {
        Some([x0, z0, x1, z1]) => {
            let left = clip_x(&front, x0, false);
            let right = clip_x(&front, x1, true);
            let mid = clip_x(&clip_x(&front, x0, true), x1, false);
            vec![left, right, clip_y(&mid, z0, false), clip_y(&mid, z1, true)]
        }
        None => vec![front],
    };
    for p in pieces.into_iter().filter(|p| ok(p)) {
        out.push((
            DormerPart::Walls,
            vertical_prism(
                owner,
                d.wall_material,
                &p,
                f.plan(0.0, t / 2.0),
                f.across,
                t / 2.0,
            ),
        ));
    }
    // The cheeks, from the front to where the eave meets the roof.
    let cheek = lift(shape.cheek_profile(below, d.kind, d.pitch));
    if ok(&cheek) {
        for x in [-d.width / 2.0 + t / 2.0, d.width / 2.0 - t / 2.0] {
            out.push((
                DormerPart::Walls,
                vertical_prism(
                    owner,
                    d.wall_material,
                    &cheek,
                    f.plan(x, 0.0),
                    f.up,
                    t / 2.0,
                ),
            ));
        }
    }
    // Its roof, of the roof's own build-up.
    if let Some(rt) = model.slab_type(roof.roof_type) {
        for plane in &shape.planes {
            let pl = plane.clone();
            for s in layered_plate(
                owner,
                &plane.outline,
                &[],
                &move |p| lz + pl.top_at(p),
                &|th| th * (1.0 + plane.slope.slope * plane.slope.slope).sqrt(),
                &rt.layers,
            ) {
                out.push((DormerPart::Roof, s));
            }
        }
    }
    // The window: a frame round the opening and its glass.
    if let Some([x0, z0, x1, z1]) = window {
        let fw = DORMER_FRAME;
        let bars = [
            [x0, z0, x1, z0 + fw],
            [x0, z1 - fw, x1, z1],
            [x0, z0 + fw, x0 + fw, z1 - fw],
            [x1 - fw, z0 + fw, x1, z1 - fw],
        ];
        for [a, b, c, e] in bars {
            let rect = vec![
                Point2::new(a, b),
                Point2::new(c, b),
                Point2::new(c, e),
                Point2::new(a, e),
            ];
            out.push((
                DormerPart::Frame,
                vertical_prism(
                    owner,
                    d.frame_material,
                    &rect,
                    f.plan(0.0, 0.065),
                    f.across,
                    0.035,
                ),
            ));
        }
        let pane = vec![
            Point2::new(x0 + fw, z0 + fw),
            Point2::new(x1 - fw, z0 + fw),
            Point2::new(x1 - fw, z1 - fw),
            Point2::new(x0 + fw, z1 - fw),
        ];
        if ok(&pane) {
            out.push((
                DormerPart::Glass,
                vertical_prism(
                    owner,
                    d.glass_material,
                    &pane,
                    f.plan(0.0, 0.068),
                    f.across,
                    0.008,
                ),
            ));
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn skylight(
    model: &Model,
    roof: &Roof,
    d: &Dormer,
    shape: &DormerShape,
    owner: SolidOwner,
    lz: f64,
    below: f64,
    out: &mut Vec<(DormerPart, Solid)>,
) {
    let f = shape.frame;
    let s = f.slope;
    let k = (1.0 + s * s).sqrt();
    let len = shape.back;
    let fw = DORMER_FRAME;
    let hw = d.width / 2.0;
    // The inner opening, the frame's width in from the footprint (along
    // the slope, measured on it).
    let inner: Vec<Point2> = [
        (-hw + fw, fw / k),
        (hw - fw, fw / k),
        (hw - fw, len - fw / k),
        (-hw + fw, len - fw / k),
    ]
    .iter()
    .map(|&(x, y)| f.plan(x, y))
    .collect();
    let _ = model;
    let top = move |p: Point2| lz + f.z0 + s * (p - f.origin).dot(f.up);
    let frame_layer = [archisim_core::Layer {
        material: d.frame_material,
        thickness: 0.06 + below / k,
        function: archisim_core::LayerFunction::Finish,
    }];
    for sol in layered_plate(
        owner,
        &shape.footprint,
        std::slice::from_ref(&inner),
        &move |p| top(p) + 0.06,
        &|t| t * k,
        &frame_layer,
    ) {
        out.push((DormerPart::Frame, sol));
    }
    let glass = [archisim_core::Layer {
        material: d.glass_material,
        thickness: 0.012,
        function: archisim_core::LayerFunction::Finish,
    }];
    for sol in layered_plate(
        owner,
        &inner,
        &[],
        &move |p| top(p) + 0.045,
        &|t| t * k,
        &glass,
    ) {
        out.push((DormerPart::Glass, sol));
    }
    let _ = roof;
}

/// The solids of a dormer.
pub fn dormer_solids(model: &Model, id: RoofId, index: usize) -> Vec<Solid> {
    dormer_parts(model, id, index)
        .into_iter()
        .map(|(_, s)| s)
        .collect()
}

/// A half-round section of radius `r` on `y = 0`, counter-clockwise:
/// the upper half (`up`, a cover) or the lower one (a gutter).
fn half_round(r: f64, facets: usize, up: bool) -> Vec<(f64, f64)> {
    let start = if up { 0.0 } else { std::f64::consts::PI };
    (0..=facets)
        .map(|i| {
            let a = start + std::f64::consts::PI * i as f64 / facets as f64;
            (r * a.cos(), r * a.sin())
        })
        .collect()
}

/// Where a line through `o` along `u` is inside a shape: intervals of `t`.
fn inside_runs(o: Point2, u: Vec2, shape: &Shape) -> Vec<(f64, f64)> {
    let mut ts = Vec::new();
    let n = u.perp_left();
    for ring in std::iter::once(&shape.0).chain(shape.1.iter()) {
        let m = ring.len();
        for i in 0..m {
            let (a, b) = (ring[i], ring[(i + 1) % m]);
            let (da, db) = ((a - o).dot(n), (b - o).dot(n));
            if (da > 0.0) != (db > 0.0) {
                let k = da / (da - db);
                ts.push((a + (b - a) * k - o).dot(u));
            }
        }
    }
    ts.sort_by(f64::total_cmp);
    ts.chunks(2)
        .filter(|c| c.len() == 2 && c[1] - c[0] > 0.05)
        .map(|c| (c[0], c[1]))
        .collect()
}

/// The tiles of a roof: on each plane, a plate per course (thicker at
/// its lower edge, so the courses step down the slope) and, for curved
/// tiles and metal sheet, the covers or seams down the slope; ridge tiles
/// over its ridges and hips.
pub fn tile_solids(model: &Model, id: RoofId) -> Vec<Solid> {
    let Some(roof) = model.roof(id) else {
        return Vec::new();
    };
    let Some(kind) = roof.covering.tiles else {
        return Vec::new();
    };
    let Some(material) = model.tile_material(roof) else {
        return Vec::new();
    };
    let owner = SolidOwner::Roof(id);
    let lz = level_z(model, roof);
    let mut out = Vec::new();
    let skin = [archisim_core::Layer {
        material,
        thickness: 0.015,
        function: archisim_core::LayerFunction::Finish,
    }];
    let (step, rib) = match kind {
        TileKind::Arabic => (0.035, Some((0.075, 6))),
        TileKind::Mixed => (0.03, Some((0.05, 4))),
        TileKind::Flat => (0.02, None),
        TileKind::Slate => (0.01, None),
        TileKind::Metal => (0.0, None),
    };
    for plane in roof.planes() {
        let Some(u) = plane.slope.direction() else {
            continue;
        };
        let s = plane.slope.slope;
        let k = (1.0 + s * s).sqrt();
        let holes = roof.plane_holes(plane.side);
        let holes: Vec<Vec<Point2>> = holes
            .into_iter()
            .filter(|h| {
                h.iter()
                    .all(|q| archisim_core::point_in_polygon(*q, &plane.outline))
            })
            .collect();
        let shape: Shape = (crate::ensure_ccw(plane.outline.clone()), holes);
        let from = plane.slope.from;
        let base = roof.base_offset;
        let roof_top = move |p: Point2| lz + base + s * (p - from).dot(u);
        let ds: Vec<f64> = plane.outline.iter().map(|p| (*p - from).dot(u)).collect();
        let (d0, d1) = ds
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &d| {
                (a.min(d), b.max(d))
            });
        let across = u.perp_left();
        let cs: Vec<f64> = plane
            .outline
            .iter()
            .map(|p| (*p - from).dot(across))
            .collect();
        let (c0, c1) = cs
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &c| {
                (a.min(c), b.max(c))
            });
        // Courses, from the eave up.
        let course = kind.course().map(|c| c / k);
        let bands: Vec<(f64, f64)> = match course {
            Some(c) => {
                let n = ((d1 - d0) / c).ceil().max(1.0) as usize;
                (0..n)
                    .map(|i| (d0 + c * i as f64, (d0 + c * (i + 1) as f64).min(d1)))
                    .collect()
            }
            None => vec![(d0, d1)],
        };
        let at = |c: f64, d: f64| from + across * c + u * d;
        for (lo, hi) in bands {
            let strip = vec![
                at(c0 - 1.0, lo),
                at(c1 + 1.0, lo),
                at(c1 + 1.0, hi),
                at(c0 - 1.0, hi),
            ];
            let span = (hi - lo).max(1e-9);
            for (outer, hs) in
                junctions::intersection(&[(strip, Vec::new())], std::slice::from_ref(&shape))
            {
                let top = move |p: Point2| {
                    let f = ((p - from).dot(u) - lo) / span;
                    roof_top(p) + 0.015 * k + step * (1.0 - f.clamp(0.0, 1.0))
                };
                out.extend(layered_plate(owner, &outer, &hs, &top, &|t| t * k, &skin));
            }
        }
        // Covers or seams down the slope.
        let (r, facets, seam) = match (rib, kind) {
            (Some((r, n)), _) => (r, n, false),
            (None, TileKind::Metal) => (0.02, 4, true),
            _ => continue,
        };
        let Some(gap) = kind.ribs() else { continue };
        let mut c = c0 + gap / 2.0;
        let section: Vec<(f64, f64)> = if seam {
            vec![(-0.012, 0.0), (0.012, 0.0), (0.012, 0.04), (-0.012, 0.04)]
        } else {
            half_round(r, facets, true)
        };
        while c < c1 {
            let o = at(c, 0.0);
            for (ta, tb) in inside_runs(o, u, &shape) {
                let (pa, pb) = (o + u * (ta + 0.02), o + u * (tb - 0.02));
                let lift = 0.015 * k + step;
                let a: P3 = [pa.x, pa.y, roof_top(pa) + lift];
                let b: P3 = [pb.x, pb.y, roof_top(pb) + lift];
                out.push(Solid {
                    owner,
                    material,
                    triangles: sweep_segment(&section, a, b),
                });
            }
            c += gap;
        }
    }
    // Ridge tiles over ridges and hips.
    let ridge = half_round(0.11, 6, true);
    for (p, q) in roof.ridge_lines() {
        let a: P3 = [p.x, p.y, lz + roof.top_at(p) + 0.01];
        let b: P3 = [q.x, q.y, lz + roof.top_at(q) + 0.01];
        if (a[0] - b[0]).hypot(a[1] - b[1]) > 1e-6 {
            out.push(Solid {
                owner,
                material,
                triangles: sweep_segment(&ridge, a, b),
            });
        }
    }
    out.retain(|s| !s.triangles.is_empty());
    out
}

/// A round section of `d` diameter, `n` sides.
fn round(d: f64, n: usize) -> Vec<Point2> {
    (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            Point2::new(d / 2.0 * a.cos(), d / 2.0 * a.sin())
        })
        .collect()
}

/// The gutters along a roof's eaves and their downpipes, down to the
/// lowest level.
pub fn gutter_solids(model: &Model, id: RoofId) -> Vec<Solid> {
    gutter_pieces(model, id)
        .into_iter()
        .map(|(_, _, s)| s)
        .collect()
}

/// The pieces of the gutters: the run (index in [`Roof::gutter_runs`]),
/// the downpipe (`None`: the gutter itself) and its solid.
pub fn gutter_pieces(model: &Model, id: RoofId) -> Vec<(usize, Option<usize>, Solid)> {
    let Some(roof) = model.roof(id) else {
        return Vec::new();
    };
    let Some(g) = &roof.covering.gutter else {
        return Vec::new();
    };
    let owner = SolidOwner::Roof(id);
    let lz = level_z(model, roof);
    let ground = model
        .levels()
        .iter()
        .map(|l| l.elevation)
        .fold(f64::INFINITY, f64::min)
        .min(lz);
    let w = g.size;
    let section: Vec<(f64, f64)> = match g.shape {
        archisim_core::GutterShape::HalfRound => half_round(w / 2.0, 8, false),
        archisim_core::GutterShape::Box => vec![
            (-w / 2.0, -w * 0.75),
            (w / 2.0, -w * 0.75),
            (w / 2.0, 0.0),
            (-w / 2.0, 0.0),
        ],
    };
    let mut out = Vec::new();
    for (ri, run) in roof.gutter_runs().into_iter().enumerate() {
        let z = lz + run.z - 0.02;
        out.push((
            ri,
            None,
            Solid {
                owner,
                material: g.material,
                triangles: sweep_segment(
                    &section,
                    [run.start.x, run.start.y, z],
                    [run.end.x, run.end.y, z],
                ),
            },
        ));
        let line = run.end - run.start;
        let len2 = line.dot(line).max(1e-12);
        for (pi, p) in run.downpipes.iter().enumerate() {
            // Its outlet: under the gutter, the nearest point.
            let t = ((*p - run.start).dot(line) / len2).clamp(0.0, 1.0);
            let outlet = run.start + line * t;
            let bottom = z - w * 0.45;
            let top = bottom - 0.25;
            let ring = round(g.downpipe, 10);
            let at = |c: Point2| ring.iter().map(|q| *q + c).collect::<Vec<_>>();
            // The bend back to the wall, then down it.
            let elbow: Vec<(f64, f64)> = ring.iter().map(|q| (q.x, q.y)).collect();
            out.push((
                ri,
                Some(pi),
                Solid {
                    owner,
                    material: g.material,
                    triangles: sweep_segment(&elbow, [outlet.x, outlet.y, bottom], [p.x, p.y, top]),
                },
            ));
            if top - ground > 0.05 {
                out.push((
                    ri,
                    Some(pi),
                    Solid {
                        owner,
                        material: g.material,
                        triangles: prism(&at(*p), ground, top),
                    },
                ));
            }
        }
    }
    out.retain(|(_, _, s)| !s.triangles.is_empty());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{CutPattern, Layer, LayerFunction, NewRoof, RoofCovering, SlabFunction};

    fn house() -> (Model, RoofId) {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let tile = m.add_material("Teja", "ceramic", [190, 100, 70], CutPattern::Solid);
        m.add_material("Monocapa", "plaster", [240, 240, 230], CutPattern::Solid);
        m.add_material("Aluminio", "aluminium", [120, 120, 120], CutPattern::Solid);
        m.add_material("Vidrio", "glass", [200, 220, 240], CutPattern::Solid);
        m.add_material("Zinc", "metal", [150, 150, 155], CutPattern::Solid);
        let t = m
            .add_slab_type(
                "Cubierta",
                SlabFunction::Roof,
                vec![Layer {
                    material: tile,
                    thickness: 0.25,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let r = m
            .add_roof(NewRoof {
                roof_type: t,
                level: l,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(10.0, 0.0),
                    Point2::new(10.0, 12.0),
                    Point2::new(0.0, 12.0),
                ],
                slopes: vec![Some(0.5), None, Some(0.5), None],
                base_offset: 3.0,
                overhang: 0.5,
            })
            .unwrap();
        (m, r)
    }

    #[test]
    fn dormers_are_closed_and_cut_the_roof() {
        let (mut m, r) = house();
        for (kind, at) in [
            (DormerKind::Gable, Point2::new(3.0, 1.0)),
            (DormerKind::Shed, Point2::new(7.0, 1.0)),
            (DormerKind::Skylight, Point2::new(5.0, 10.0)),
        ] {
            m.add_dormer(r, kind, at).unwrap();
        }
        for i in 0..3 {
            let parts = dormer_parts(&m, r, i);
            assert!(!parts.is_empty(), "dormer {i}");
            for (_, s) in &parts {
                assert!(s.volume() > 0.0, "dormer {i}: {:?}", s.material);
            }
        }
        // A window in the front of the gabled one.
        assert!(dormer_parts(&m, r, 0)
            .iter()
            .any(|(p, _)| *p == DormerPart::Glass));
        // The roof loses the dormers' footprints.
        let before: f64 = {
            let (m2, r2) = house();
            crate::solid::roof_solids(&m2, r2)
                .iter()
                .map(|s| s.volume())
                .sum()
        };
        let after: f64 = crate::solid::roof_solids(&m, r)
            .iter()
            .map(|s| s.volume())
            .sum();
        assert!(after < before - 0.1, "{after} < {before}");
    }

    #[test]
    fn tiles_and_gutters() {
        let (mut m, r) = house();
        let g = m.default_gutter().unwrap();
        m.set_roof_covering(
            r,
            RoofCovering {
                tiles: Some(TileKind::Arabic),
                tile_material: None,
                gutter: Some(g),
            },
        )
        .unwrap();
        let tiles = tile_solids(&m, r);
        assert!(tiles.len() > 50);
        let tris: usize = tiles.iter().map(|s| s.triangles.len()).sum();
        // Light enough for the 3D view.
        assert!(tris < 20_000, "{tris}");
        let gutters = gutter_solids(&m, r);
        assert!(!gutters.is_empty());
        // Downpipes reach the ground.
        let low = gutters
            .iter()
            .flat_map(|s| s.triangles.iter().flatten())
            .map(|p| p[2])
            .fold(f64::INFINITY, f64::min);
        assert!(low.abs() < 1e-9);
    }
}
