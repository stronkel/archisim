//! False ceilings in 3D (M7): each layer of its type a flat prism, the
//! board at the face seen from the room (`Ceiling::height` over the level)
//! and the others above it, with its holes through all of them.

use archisim_core::{CeilingId, Model, Point2};

use crate::ensure_ccw;
use crate::solid::{triangulate, with_holes, Solid, SolidOwner, P3};

/// The heights of each layer of a false ceiling, from the top down:
/// (bottom, top) over the level's datum, and the layer's material.
pub fn ceiling_layers(model: &Model, id: CeilingId) -> Vec<(f64, f64, archisim_core::MaterialId)> {
    let Some(c) = model.ceiling(id) else {
        return Vec::new();
    };
    let Some(t) = model.slab_type(c.ceiling_type) else {
        return Vec::new();
    };
    let level = model.level(c.level).map_or(0.0, |l| l.elevation);
    let mut top = level + c.height + t.thickness();
    t.layers
        .iter()
        .filter(|l| l.thickness > 1e-6)
        .map(|l| {
            let bottom = top - l.thickness;
            let layer = (bottom, top, l.material);
            top = bottom;
            layer
        })
        .collect()
}

/// The solids of a false ceiling, one per layer.
pub fn ceiling_solids(model: &Model, id: CeilingId) -> Vec<Solid> {
    let Some(c) = model.ceiling(id) else {
        return Vec::new();
    };
    ceiling_layers(model, id)
        .into_iter()
        .map(|(z0, z1, material)| Solid {
            owner: SolidOwner::Ceiling(id),
            material,
            triangles: plate(&c.outline, &c.holes, z0, z1),
        })
        .collect()
}

/// A flat prism with holes, closed and with its faces outwards.
pub fn plate(outline: &[Point2], holes: &[Vec<Point2>], z0: f64, z1: f64) -> Vec<[P3; 3]> {
    if outline.len() < 3 || z1 - z0 <= 1e-9 {
        return Vec::new();
    }
    let at = |p: Point2, z: f64| [p.x, p.y, z];
    let poly = with_holes(outline, holes);
    let mut tris = Vec::new();
    for [a, b, c] in triangulate(&poly) {
        tris.push([at(poly[a], z1), at(poly[b], z1), at(poly[c], z1)]);
        tris.push([at(poly[a], z0), at(poly[c], z0), at(poly[b], z0)]);
    }
    // Sides: the outline counter-clockwise, the holes clockwise, so the
    // outside is always on the right of each edge.
    let mut rings = vec![ensure_ccw(outline.to_vec())];
    rings.extend(holes.iter().map(|h| {
        let mut h = ensure_ccw(h.clone());
        h.reverse();
        h
    }));
    for ring in &rings {
        for i in 0..ring.len() {
            let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
            if p.approx_eq(q) {
                continue;
            }
            tris.push([at(p, z0), at(q, z0), at(q, z1)]);
            tris.push([at(p, z0), at(q, z1), at(p, z1)]);
        }
    }
    tris
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{CutPattern, Layer, LayerFunction, NewCeiling, SlabFunction};

    #[test]
    fn a_false_ceiling_hangs_at_its_height() {
        let mut m = Model::new("T");
        let lv = m.add_level("PB", 0.0);
        let board = m.add_material("Placa", "plaster", [240, 240, 240], CutPattern::Solid);
        let wool = m.add_material("Lana", "insulation", [230, 220, 120], CutPattern::Solid);
        let t = m
            .add_slab_type(
                "Techo",
                SlabFunction::Ceiling,
                vec![
                    Layer {
                        material: wool,
                        thickness: 0.04,
                        function: LayerFunction::Insulation,
                    },
                    Layer {
                        material: board,
                        thickness: 0.015,
                        function: LayerFunction::Finish,
                    },
                ],
            )
            .unwrap();
        let id = m
            .add_ceiling(NewCeiling {
                ceiling_type: t,
                level: lv,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(4.0, 0.0),
                    Point2::new(4.0, 3.0),
                    Point2::new(0.0, 3.0),
                ],
                height: 2.5,
            })
            .unwrap();
        let hatch = vec![
            Point2::new(1.0, 1.0),
            Point2::new(1.6, 1.0),
            Point2::new(1.6, 1.6),
            Point2::new(1.0, 1.6),
        ];
        m.edit_ceiling(id, |c| c.holes = vec![hatch]).unwrap();
        let solids = ceiling_solids(&m, id);
        assert_eq!(solids.len(), 2);
        // The board below, at 2,50; the wool above it.
        assert_eq!(solids[1].material, board);
        let zs = |s: &Solid| {
            let z: Vec<f64> = s.triangles.iter().flatten().map(|p| p[2]).collect();
            (
                z.iter().copied().fold(f64::INFINITY, f64::min),
                z.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            )
        };
        let (b0, b1) = zs(&solids[1]);
        assert!((b0 - 2.5).abs() < 1e-9 && (b1 - 2.515).abs() < 1e-9);
        let (w0, w1) = zs(&solids[0]);
        assert!((w0 - 2.515).abs() < 1e-9 && (w1 - 2.555).abs() < 1e-9);
        // Closed: its volume is its area by its thickness.
        let area = 12.0 - 0.36;
        assert!((solids[1].volume() - area * 0.015).abs() < 1e-9);
        assert!((solids[0].volume() - area * 0.04).abs() < 1e-9);
    }
}
