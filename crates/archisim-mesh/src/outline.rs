//! What a model covers seen from above: the union of its triangles in
//! plan, simplified, for the symbol of its object type.

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

use crate::Mesh;

type Ring = Vec<[f64; 2]>;

fn area(r: &[[f64; 2]]) -> f64 {
    let n = r.len();
    (0..n)
        .map(|i| {
            let (a, b) = (r[i], r[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

/// Douglas–Peucker on a closed ring.
fn simplify_ring(r: &[[f64; 2]], tol: f64) -> Ring {
    fn dist(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l2 = dx * dx + dy * dy;
        let t = if l2 < 1e-24 {
            0.0
        } else {
            (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0)
        };
        let (x, y) = (a[0] + dx * t - p[0], a[1] + dy * t - p[1]);
        (x * x + y * y).sqrt()
    }
    fn run(pts: &[[f64; 2]], tol: f64, out: &mut Ring) {
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        let (i, d) = pts[1..pts.len() - 1]
            .iter()
            .enumerate()
            .map(|(i, p)| (i + 1, dist(*p, a, b)))
            .fold((0, 0.0), |m, x| if x.1 > m.1 { x } else { m });
        if d > tol && i > 0 {
            run(&pts[..=i], tol, out);
            run(&pts[i..], tol, out);
        } else {
            out.push(a);
        }
    }
    if r.len() < 4 {
        return r.to_vec();
    }
    // Split at the point farthest from the first, so the ends are fixed.
    let far = (1..r.len())
        .max_by(|&i, &j| {
            let d = |k: usize| (r[k][0] - r[0][0]).hypot(r[k][1] - r[0][1]);
            d(i).total_cmp(&d(j))
        })
        .unwrap_or(1);
    let mut out = Vec::new();
    run(&r[..=far], tol, &mut out);
    let mut back: Ring = r[far..].to_vec();
    back.push(r[0]);
    run(&back, tol, &mut out);
    out
}

/// The outline of `mesh` in plan: each shape as its outer ring
/// (counter-clockwise) and holes, simplified to `tol` metres; pieces
/// smaller than a hundredth of the whole are left out.
pub fn outline(mesh: &Mesh, tol: f64) -> Vec<(Ring, Vec<Ring>)> {
    let light = crate::simplify(mesh, 3_000);
    let mut tris: Vec<Ring> = Vec::new();
    for g in &light.groups {
        for t in &g.triangles {
            let q = t.map(|i| {
                let p = light.positions[i as usize];
                [p[0] as f64, p[1] as f64]
            });
            let a = area(&q);
            if a.abs() < 1e-10 {
                continue;
            }
            tris.push(if a > 0.0 {
                q.to_vec()
            } else {
                vec![q[0], q[2], q[1]]
            });
        }
    }
    if tris.is_empty() {
        return Vec::new();
    }
    let shapes = tris.overlay(&Vec::<Ring>::new(), OverlayRule::Union, FillRule::NonZero);
    let total: f64 = shapes
        .iter()
        .filter_map(|s| s.first())
        .map(|r| area(r).abs())
        .sum();
    let mut out = Vec::new();
    for s in shapes {
        let mut it = s.into_iter();
        let Some(outer) = it.next() else { continue };
        if area(&outer).abs() < total * 0.01 {
            continue;
        }
        let mut outer = simplify_ring(&outer, tol);
        if outer.len() < 3 {
            continue;
        }
        if area(&outer) < 0.0 {
            outer.reverse();
        }
        let holes = it
            .filter(|h| area(h).abs() >= total * 0.01)
            .map(|h| simplify_ring(&h, tol))
            .filter(|h| h.len() >= 3)
            .collect();
        out.push((outer, holes));
    }
    out
}
