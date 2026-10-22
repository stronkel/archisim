//! The accessories of wall and slab types in 3D (bloque 5 de la revisión
//! del 22/10/2026): each sweep of a wall type runs along that face of
//! every straight wall of the type, from end to end of the face (a little
//! further where the wall joins another, so corners close), broken where
//! a door or window crosses its height; a slab type's edge runs round the
//! outline of every level slab of the type. The meshes are closed, with
//! `z` in the model.

use archisim_core::{MassMesh, MaterialId, Model, Point2, SlabId, SweepKind, SweepSide, WallId};

use crate::{wall_geometry, EndCut};

/// One run of an accessory.
#[derive(Debug, Clone)]
pub struct SweepPiece {
    /// Which sweep of the type (for a slab's edge, 0).
    pub index: usize,
    pub kind: Option<SweepKind>,
    pub material: MaterialId,
    pub mesh: MassMesh,
}

fn y_range(p: &[Point2]) -> (f64, f64) {
    p.iter().fold((f64::MAX, f64::MIN), |(lo, hi), q| {
        (lo.min(q.y), hi.max(q.y))
    })
}

/// The runs of the accessories of a wall's type.
pub fn wall_sweep_pieces(model: &Model, id: WallId) -> Vec<SweepPiece> {
    let Some(wall) = model.wall(id) else {
        return Vec::new();
    };
    let Some(wt) = model.wall_type(wall.wall_type) else {
        return Vec::new();
    };
    if wt.sweeps.is_empty() {
        return Vec::new();
    }
    let Some(g) = wall_geometry(model, id) else {
        return Vec::new();
    };
    if g.arc.is_some() {
        return Vec::new();
    }
    let (a, b) = g.axis;
    let Some(u) = (b - a).normalized() else {
        return Vec::new();
    };
    let n = u.perp_left();
    let base = model.level(wall.level).map_or(0.0, |l| l.elevation) + wall.base_offset;
    let h = wall.height;
    let openings = crate::openings_in_wall(model, id);
    let mut out = Vec::new();
    for (index, s) in wt.sweeps.iter().enumerate() {
        let face = match s.side {
            SweepSide::Interior => g.interior_offset,
            _ => g.exterior_offset,
        };
        // Where the face runs, mitred corners included.
        let on: Vec<f64> = g
            .outline
            .iter()
            .filter(|p| ((**p - a).dot(n) - face).abs() < 1e-6)
            .map(|p| (*p - a).dot(u))
            .collect();
        let lo = on.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = on.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if !lo.is_finite() || hi - lo < 0.01 {
            continue;
        }
        let reach = s.profile.iter().map(|p| p.x.abs()).fold(0.0, f64::max);
        let joined = |c: &EndCut| !matches!(c, EndCut::Square);
        let lo = if joined(&g.start_cut) { lo - reach } else { lo };
        let hi = if joined(&g.end_cut) { hi + reach } else { hi };
        let (ylo, yhi) = y_range(&s.profile);
        let z0 = match (s.side, s.from_top) {
            (SweepSide::Top, _) => base + h + s.offset,
            (_, true) => base + h - s.offset - yhi,
            (_, false) => base + s.offset,
        };
        // Broken where a door or window crosses it.
        let (zlo, zhi) = (z0 + ylo - base, z0 + yhi - base);
        let mut runs = vec![(lo, hi)];
        if s.side != SweepSide::Top {
            for o in &openings {
                if o.head <= zlo + 1e-6 || o.sill >= zhi - 1e-6 {
                    continue;
                }
                runs = runs
                    .into_iter()
                    .flat_map(|(r0, r1)| {
                        let mut v = Vec::new();
                        if o.t0 > r0 {
                            v.push((r0, o.t0.min(r1)));
                        }
                        if o.t1 < r1 {
                            v.push((o.t1.max(r0), r1));
                        }
                        v
                    })
                    .filter(|(r0, r1)| r1 - r0 > 0.01)
                    .collect();
            }
        }
        // Out of the face is to the left of the path.
        let profile: Vec<Point2> = match s.side {
            SweepSide::Top => s.profile.iter().map(|p| Point2::new(-p.x, p.y)).collect(),
            _ => s.profile.clone(),
        };
        for (r0, r1) in runs {
            let at = |t: f64| a + u * t + n * face;
            let path = match s.side {
                SweepSide::Interior => [at(r1), at(r0)],
                _ => [at(r0), at(r1)],
            };
            let mut mesh = crate::sculpt::sweep(&path, &profile);
            for p in &mut mesh.positions {
                p[2] += z0;
            }
            out.push(SweepPiece {
                index,
                kind: Some(s.kind),
                material: s.material,
                mesh,
            });
        }
    }
    out
}

/// The edge of a slab's type round its outline (level slabs only).
pub fn slab_edge_piece(model: &Model, id: SlabId) -> Option<SweepPiece> {
    let slab = model.slab(id)?;
    let st = model.slab_type(slab.slab_type)?;
    let edge = st.edge.as_ref()?;
    if slab.slope.is_some() {
        return None;
    }
    let top = model.level(slab.level).map_or(0.0, |l| l.elevation) + slab.top_offset;
    // Clockwise: out of the slab is to the left.
    let mut path = slab.outline.clone();
    if archisim_core::signed_area(&path) > 0.0 {
        path.reverse();
    }
    let mut mesh = crate::sculpt::sweep_closed(&path, &edge.profile);
    for p in &mut mesh.positions {
        p[2] += top;
    }
    Some(SweepPiece {
        index: 0,
        kind: None,
        material: edge.material,
        mesh,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    #[test]
    fn skirtings_stop_at_doors_and_edges_go_round() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let mat = m.add_material("Yeso", "plaster", [240, 240, 240], CutPattern::Solid);
        let layer = Layer {
            material: mat,
            thickness: 0.2,
            function: LayerFunction::Structure,
        };
        let wt = m
            .add_wall_type("Muro", WallFunction::Interior, vec![layer.clone()])
            .unwrap();
        m.set_wall_type_sweeps(
            wt,
            vec![WallSweep {
                kind: SweepKind::Skirting,
                side: SweepSide::Interior,
                offset: 0.0,
                from_top: false,
                profile: sweep_preset(SweepKind::Skirting, 0.2),
                material: mat,
            }],
        )
        .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: wt,
                level: pb,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(5.0, 0.0),
                reference: ReferenceLine::Center,
                height: 2.7,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        let pieces = wall_sweep_pieces(&m, w);
        assert_eq!(pieces.len(), 1);
        let v = crate::forms::mesh_volume(&pieces[0].mesh);
        assert!(v > 0.0 && v < 5.0 * 0.015 * 0.08 + 1e-6, "{v}");
        // On the interior face, 10 cm to the right of the axis.
        assert!(pieces[0].mesh.positions.iter().all(|p| p[1] < -0.099));
        // A door in the middle: two runs.
        let door = m.opening_types().find(|t| t.is_door()).map(|t| t.id);
        if let Some(door) = door {
            m.add_opening(NewOpening {
                opening_type: door,
                wall: w,
                along: 2.5,
                sill: 0.0,
                opens_to: WallSide::Interior,
                hand: Hand::Left,
                setback: None,
            })
            .unwrap();
            assert_eq!(wall_sweep_pieces(&m, w).len(), 2);
        }
        let st = m
            .add_slab_type("Forjado", SlabFunction::Floor, vec![layer])
            .unwrap();
        m.set_slab_type_edge(
            st,
            Some(SlabEdge {
                profile: slab_edge_preset(false),
                material: mat,
            }),
        )
        .unwrap();
        let s = m
            .add_slab(NewSlab {
                slab_type: st,
                level: pb,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(4.0, 0.0),
                    Point2::new(4.0, 3.0),
                    Point2::new(0.0, 3.0),
                ],
                top_offset: 3.0,
            })
            .unwrap();
        let e = slab_edge_piece(&m, s).unwrap();
        let v = crate::forms::mesh_volume(&e.mesh);
        assert!(v > 0.0, "{v}");
        // It stands out of the slab all round.
        let max_x = e
            .mesh
            .positions
            .iter()
            .map(|p| p[0])
            .fold(f64::MIN, f64::max);
        assert!((max_x - 4.2).abs() < 1e-6, "{max_x}");
    }
}
