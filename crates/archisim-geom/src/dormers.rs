//! Dormers in 3D (`archisim_core::Dormer`): a closed body of the
//! material of its walls (front, cheeks, and its underside on the main
//! roof), a skin of the roof's covering over its roof, standing out a
//! little at the front and sides, and a window in its front.

use archisim_core::{Dormer, DormerKind, DormerShape, MaterialId, Model, RoofId};

use crate::solid::{Solid, SolidOwner, P3};

/// How far the dormer's roof stands out at the front and sides, metres.
const EAVE: f64 = 0.15;

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

fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn centroid(pts: &[P3]) -> P3 {
    let n = pts.len().max(1) as f64;
    let s = pts
        .iter()
        .fold([0.0; 3], |s, p| [s[0] + p[0], s[1] + p[1], s[2] + p[2]]);
    [s[0] / n, s[1] / n, s[2] / n]
}

/// The triangles of a convex polyhedron given by its faces (each a convex
/// polygon in any order), every one turned outwards.
fn closed(faces: &[Vec<P3>]) -> Vec<[P3; 3]> {
    let all: Vec<P3> = faces.iter().flatten().copied().collect();
    let c = centroid(&all);
    let mut out = Vec::new();
    for f in faces {
        if f.len() < 3 {
            continue;
        }
        let fc = centroid(f);
        let mut tris: Vec<[P3; 3]> = (1..f.len() - 1).map(|i| [f[0], f[i], f[i + 1]]).collect();
        let n = tris
            .iter()
            .map(|[a, b, d]| cross(sub(*b, *a), sub(*d, *a)))
            .fold([0.0; 3], |s, v| [s[0] + v[0], s[1] + v[1], s[2] + v[2]]);
        if dot(n, sub(fc, c)) < 0.0 {
            for t in &mut tris {
                t.swap(1, 2);
            }
        }
        out.extend(tris);
    }
    out
}

/// A plate over a convex polygon, `thickness` thick along `up`.
fn plate(poly: &[P3], up: P3, thickness: f64) -> Vec<[P3; 3]> {
    let len = dot(up, up).sqrt().max(1e-12);
    let u = [
        up[0] / len * thickness,
        up[1] / len * thickness,
        up[2] / len * thickness,
    ];
    let top: Vec<P3> = poly
        .iter()
        .map(|p| [p[0] + u[0], p[1] + u[1], p[2] + u[2]])
        .collect();
    let n = poly.len();
    let mut faces = vec![poly.to_vec(), top.clone()];
    for i in 0..n {
        let j = (i + 1) % n;
        faces.push(vec![poly[i], poly[j], top[j], top[i]]);
    }
    closed(&faces)
}

/// The meshes of one dormer: its body, the skin of its roof and its
/// window's glass, in the model (`z` above project zero).
pub struct DormerMeshes {
    pub body: Vec<[P3; 3]>,
    pub roof: Vec<[P3; 3]>,
    pub glass: Vec<[P3; 3]>,
}

pub fn dormer_meshes(d: &Dormer, s: &DormerShape, level: f64, covering: f64) -> DormerMeshes {
    let f = &s.frame;
    let w = d.width / 2.0;
    let at = |x: f64, t: f64, z: f64| -> P3 {
        let p = f.at(x, t);
        [p.x, p.y, level + z]
    };
    let ground = |x: f64, t: f64| at(x, t, f.roof_z(t));
    let (body, roof_faces): (Vec<Vec<P3>>, Vec<(Vec<P3>, P3)>) = match d.kind {
        DormerKind::Shed => {
            let back = f.roof_z(s.depth);
            let ftl = at(-w, 0.0, s.front_top);
            let ftr = at(w, 0.0, s.front_top);
            let bl = at(-w, s.depth, back);
            let br = at(w, s.depth, back);
            let (fbl, fbr) = (ground(-w, 0.0), ground(w, 0.0));
            let body = vec![
                vec![fbl, fbr, ftr, ftl],
                vec![ftl, ftr, br, bl],
                vec![fbl, ftl, bl],
                vec![fbr, br, ftr],
                vec![fbl, bl, br, fbr],
            ];
            // The roof, standing out at the front and sides.
            let e = w + EAVE;
            let skin = vec![
                at(-e, -EAVE, s.front_top - d.pitch * EAVE),
                at(e, -EAVE, s.front_top - d.pitch * EAVE),
                at(e, s.depth, back),
                at(-e, s.depth, back),
            ];
            (body, vec![(skin, [0.0, 0.0, 1.0])])
        }
        DormerKind::Gable => {
            let ftl = at(-w, 0.0, s.front_top);
            let ftr = at(w, 0.0, s.front_top);
            let fr = at(0.0, 0.0, s.ridge);
            let rb = at(0.0, s.depth, s.ridge);
            let el = at(-w, s.eave_depth, s.front_top);
            let er = at(w, s.eave_depth, s.front_top);
            let (fbl, fbr) = (ground(-w, 0.0), ground(w, 0.0));
            let body = vec![
                vec![fbl, fbr, ftr, fr, ftl],
                vec![ftl, fr, rb, el],
                vec![fr, ftr, er, rb],
                vec![fbl, ftl, el],
                vec![fbr, er, ftr],
                vec![fbl, el, rb, er, fbr],
            ];
            let e = w + EAVE;
            let low = s.front_top - d.pitch * EAVE;
            let left = vec![
                at(-e, -EAVE, low),
                at(0.0, -EAVE, s.ridge),
                at(0.0, s.depth, s.ridge),
                at(-e, s.eave_depth, low),
            ];
            let right = vec![
                at(0.0, -EAVE, s.ridge),
                at(e, -EAVE, low),
                at(e, s.eave_depth, low),
                at(0.0, s.depth, s.ridge),
            ];
            (
                body,
                vec![(left, [0.0, 0.0, 1.0]), (right, [0.0, 0.0, 1.0])],
            )
        }
    };
    let roof = roof_faces
        .iter()
        .flat_map(|(poly, _)| {
            // Thickness along the face's upward normal.
            let n = cross(sub(poly[1], poly[0]), sub(poly[2], poly[0]));
            let n = if n[2] < 0.0 { [-n[0], -n[1], -n[2]] } else { n };
            plate(poly, n, covering)
        })
        .collect();
    let glass = if d.window {
        let gw = (d.width * 0.6).min(d.width - 0.2).max(0.1) / 2.0;
        let (z0, z1) = (f.z0 + d.height * 0.2, f.z0 + d.height * 0.85);
        let pane = vec![
            at(-gw, -0.01, z0),
            at(gw, -0.01, z0),
            at(gw, -0.01, z1),
            at(-gw, -0.01, z1),
        ];
        plate(&pane, [f.down.x, f.down.y, 0.0], 0.03)
    } else {
        Vec::new()
    };
    DormerMeshes {
        body: closed(&body),
        roof,
        glass,
    }
}

/// The solids of a roof's dormers.
pub fn dormer_solids(model: &Model, id: RoofId) -> Vec<crate::solid::Solid> {
    let Some(roof) = model.roof(id) else {
        return Vec::new();
    };
    let level = model.level(roof.level).map_or(0.0, |l| l.elevation);
    let stype = model.slab_type(roof.roof_type);
    let covering_material: Option<MaterialId> =
        stype.and_then(|t| t.layers.first()).map(|l| l.material);
    let covering = stype
        .and_then(|t| t.layers.first())
        .map_or(0.05, |l| l.thickness.clamp(0.02, 0.12));
    let glass = model
        .materials()
        .find(|m| m.category == "glass")
        .map(|m| m.id);
    let mut out = Vec::new();
    for d in &roof.dormers {
        let Ok(shape) = roof.dormer_shape(d) else {
            continue;
        };
        let m = dormer_meshes(d, &shape, level, covering);
        out.push(Solid {
            owner: SolidOwner::Roof(id),
            material: d.wall_material,
            triangles: m.body,
        });
        out.push(Solid {
            owner: SolidOwner::Roof(id),
            material: covering_material.unwrap_or(d.wall_material),
            triangles: m.roof,
        });
        if let (Some(g), false) = (glass, m.glass.is_empty()) {
            out.push(Solid {
                owner: SolidOwner::Roof(id),
                material: g,
                triangles: m.glass,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    #[test]
    fn dormer_solids_are_closed_and_stand_on_the_roof() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let mat = m.add_material("Teja", "ceramic", [180, 80, 60], CutPattern::Solid);
        m.add_material("Vidrio", "glass", [200, 220, 240], CutPattern::Solid);
        let st = m
            .add_slab_type(
                "Cubierta",
                SlabFunction::Roof,
                vec![Layer {
                    material: mat,
                    thickness: 0.25,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let r = m
            .add_roof(NewRoof {
                roof_type: st,
                level: l,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(10.0, 0.0),
                    Point2::new(10.0, 8.0),
                    Point2::new(0.0, 8.0),
                ],
                slopes: vec![Some(0.6), None, Some(0.6), None],
                base_offset: 3.0,
                overhang: 0.4,
            })
            .unwrap();
        for kind in [DormerKind::Shed, DormerKind::Gable] {
            let d = Dormer {
                position: Point2::new(if kind == DormerKind::Shed { 3.0 } else { 7.0 }, 1.0),
                width: 1.6,
                height: 1.2,
                kind,
                pitch: if kind == DormerKind::Shed { 0.1 } else { 0.5 },
                wall_material: mat,
                window: true,
            };
            m.add_dormer(r, d).unwrap();
        }
        let solids = dormer_solids(&m, r);
        // Body, roof and glass of each.
        assert_eq!(solids.len(), 6);
        for s in &solids {
            assert!(s.volume() > 0.0, "{:?}", s.material);
        }
        // The shed body: a wedge 1,6 wide, 1,2 high at the front, 2,4 deep.
        let v = solids[0].volume();
        assert!((v - 1.6 * 1.2 * 2.4 / 2.0).abs() < 1e-6, "{v}");
    }
}
