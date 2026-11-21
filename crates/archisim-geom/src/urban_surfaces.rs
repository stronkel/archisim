//! The 3D of the urban surfaces (N5): the paving inside the edge, the edge
//! round it (a kerb standing with it, or a flush gutter or band) and the
//! paint of a car park's bays, all sitting on what is under them (the
//! ground or the street) and raised by the surface's height.

use archisim_core::{Model, Point2, SurfaceEdge, SurfaceId};

use crate::solid::{draped_prism, Solid, SolidOwner, P3};

/// How deep the paving goes under its top, metres.
pub const DEPTH: f64 = 0.2;

/// The solids of an urban surface.
pub fn surface_solids(model: &Model, id: SurfaceId) -> Vec<Solid> {
    let Some(s) = model.surface(id) else {
        return Vec::new();
    };
    let base = model.level(s.level).map_or(0.0, |l| l.elevation);
    let ground = |q: Point2| base + model.surface_at(s.level, q);
    let top = |q: Point2| ground(q) + s.raise;
    let mut out = Vec::new();
    let inner = s.inner();
    let hole_edges = s.hole_edges();
    let paved = if hole_edges.is_empty() {
        inner.clone()
    } else {
        crate::solid::with_holes(&inner, &hole_edges)
    };
    out.push(Solid {
        owner: SolidOwner::Surface(id),
        material: s.material,
        triangles: draped_prism(&paved, |q| (ground(q) - DEPTH, top(q))),
    });
    if s.edge != SurfaceEdge::None && s.edge_width > 0.0 && inner.len() == s.outline.len() {
        let n = s.outline.len();
        let mut tris: Vec<[P3; 3]> = Vec::new();
        for i in 0..n {
            let j = (i + 1) % n;
            let quad = [s.outline[i], s.outline[j], inner[j], inner[i]];
            // A kerb stands a little over the paving it holds.
            let lip = if s.edge == SurfaceEdge::Kerb {
                0.02
            } else {
                0.0
            };
            tris.extend(draped_prism(&quad, |q| {
                (ground(q) - DEPTH - 0.05, top(q) + lip)
            }));
        }
        // And round its holes.
        for (h, e) in s.holes.iter().zip(&hole_edges) {
            if h.len() != e.len() {
                continue;
            }
            let h = crate::ensure_ccw(h.clone());
            let e = crate::ensure_ccw(e.clone());
            let m = h.len();
            for i in 0..m {
                let j = (i + 1) % m;
                let quad = [e[i], e[j], h[j], h[i]];
                let lip = if s.edge == SurfaceEdge::Kerb {
                    0.02
                } else {
                    0.0
                };
                tris.extend(draped_prism(&quad, |q| {
                    (ground(q) - DEPTH - 0.05, top(q) + lip)
                }));
            }
        }
        out.push(Solid {
            owner: SolidOwner::Surface(id),
            material: s.edge_material,
            triangles: tris,
        });
    }
    let bays = s.bay_lines();
    if !bays.is_empty() {
        let paint = model
            .materials()
            .find(|m| m.category == "paint")
            .map_or(s.edge_material, |m| m.id);
        let tris: Vec<[P3; 3]> = bays
            .iter()
            .flat_map(|p| draped_prism(p, |q| (top(q), top(q) + 0.004)))
            .collect();
        out.push(Solid {
            owner: SolidOwner::Surface(id),
            material: paint,
            triangles: tris,
        });
    }
    // The bays for people with reduced mobility: blue, with the white of
    // their symbol and transfer area over it.
    let (blue, white) = s.accessible_paint();
    if !blue.is_empty() {
        let find = |rgb: [u8; 3]| {
            model
                .materials()
                .find(|m| m.category == "paint" && m.cut_color == rgb)
                .map(|m| m.id)
        };
        let paint_white = model
            .materials()
            .find(|m| m.category == "paint")
            .map_or(s.edge_material, |m| m.id);
        let paint_blue = find([40, 96, 176]).unwrap_or(paint_white);
        for (polys, mat, lift) in [(blue, paint_blue, 0.003), (white, paint_white, 0.005)] {
            let tris: Vec<[P3; 3]> = polys
                .iter()
                .flat_map(|p| draped_prism(p, |q| (top(q), top(q) + lift)))
                .collect();
            out.push(Solid {
                owner: SolidOwner::Surface(id),
                material: mat,
                triangles: tris,
            });
        }
    }
    out.retain(|x| !x.triangles.is_empty());
    out
}
