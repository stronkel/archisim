//! `archisim nudos`: the nodes of the streets changed by hand (N1,
//! `docs/TERRITORIO.md` §2) — a grid of four streets with one crossing of
//! each kind (crossing with give-way signs and bump-outs on the main
//! street, raised and flat, shared surface without markings and no
//! node), a T with chamfered corners, a stop and a longer setback, an L
//! with a sharp corner and no sidewalk at one corner, a dead end with its
//! turning circle, a bend and a street that narrows without a seam. In
//! plan, in 3D and in the IFC (whose `ArchiSim_Road` sets keep the nodes'
//! settings, read back on import).

use std::path::Path;

use archisim_core::{
    ArmSettings, ArmSign, CornerSettings, CornerShape, LevelId, MaterialId, Model, NewRoad,
    NewWall, NodeKind, NodeSettings, Point2, ReferenceLine, RoadSection, WallTop,
};
use archisim_geom::crossroads::{self, Arm, Junction};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn material(m: &Model, category: &str) -> Option<MaterialId> {
    m.materials().find(|x| x.category == category).map(|x| x.id)
}

/// The node at `p` changed by `f` (given its arms).
fn change(
    m: &mut Model,
    level: LevelId,
    p: Point2,
    f: impl FnOnce(&mut NodeSettings, &[Arm]),
) -> Res {
    let key = crossroads::node_key_at(m, level, p, 5.0).ok_or("no hay nudo")?;
    let arms = crossroads::node_arms(m, &key);
    let mut n = m
        .node(&key)
        .cloned()
        .unwrap_or_else(|| NodeSettings::new(key));
    f(&mut n, &arms);
    m.set_node(n)?;
    Ok(())
}

pub fn build() -> Res<Model> {
    let mut m = Model::new("Nudos de ArchiSim");
    let pb = m.add_level("Planta baja", 0.0);
    let lib = archisim_library::add_library(&mut m)?;
    let asphalt = material(&m, "asphalt")
        .or_else(|| material(&m, "concrete"))
        .ok_or("sin asfalto")?;
    let stone = material(&m, "stone").unwrap_or(asphalt);
    let paving = material(&m, "ceramic").unwrap_or(stone);
    let paint = m.add_material(
        "Pintura vial blanca",
        "paint",
        [245, 245, 240],
        archisim_core::CutPattern::Solid,
    );
    let usual = RoadSection {
        markings: Some(paint),
        ..RoadSection::default()
    };
    let wide = RoadSection {
        markings: Some(paint),
        ..RoadSection::classic(9.0, 2.2, 0.15, 4.0)
    };
    let add_with = |m: &mut Model, name: &str, axis: Vec<Point2>, section: &RoadSection| -> Res {
        m.add_road(NewRoad {
            name: name.into(),
            level: pb,
            axis,
            section: section.clone(),
            top_offset: 0.0,
            profile: Vec::new(),
            changes: Vec::new(),
            pavement: asphalt,
            kerb_material: stone,
            sidewalk_material: paving,
        })?;
        Ok(())
    };
    let add = |m: &mut Model, name: &str, a: Point2, b: Point2| -> Res {
        add_with(m, name, vec![a, b], &usual)
    };
    let p = Point2::new;
    // A grid of two streets each way: four crossings.
    add(&mut m, "Calle Mayor", p(-80.0, 0.0), p(80.0, 0.0))?;
    add(&mut m, "Calle Real", p(-80.0, 60.0), p(80.0, 60.0))?;
    add(&mut m, "Calle del Olmo", p(-40.0, -40.0), p(-40.0, 100.0))?;
    add(&mut m, "Calle del Pino", p(40.0, -40.0), p(40.0, 100.0))?;
    // A T on the Calle Mayor and an L at the end of two short streets.
    add(&mut m, "Calle del Sauce", p(0.0, 0.0), p(0.0, -60.0))?;
    add(&mut m, "Calle Alta", p(120.0, 100.0), p(160.0, 100.0))?;
    add(&mut m, "Calle Baja", p(120.0, 100.0), p(120.0, 60.0))?;
    // A bend of two streets, and a wide avenue that goes on narrower.
    add(&mut m, "Calle del Río", p(120.0, -40.0), p(160.0, -40.0))?;
    add(&mut m, "Ronda del Río", p(160.0, -40.0), p(185.0, -5.0))?;
    add_with(
        &mut m,
        "Avenida Ancha",
        vec![p(120.0, 30.0), p(170.0, 30.0)],
        &wide,
    )?;
    add(&mut m, "Calle Estrecha", p(170.0, 30.0), p(230.0, 30.0))?;
    // One crossing of each kind: the raised one flat, the shared one
    // without markings.
    change(&mut m, pb, p(40.0, 0.0), |n, _| {
        n.kind = Some(NodeKind::Raised);
        n.flat = Some(true);
    })?;
    change(&mut m, pb, p(-40.0, 60.0), |n, _| {
        n.kind = Some(NodeKind::Shared);
        n.hide_marks = Some(true);
    })?;
    // The Calle Mayor rules where the Calle del Olmo crosses it: give way
    // on the Olmo and bump-outs on its four corners.
    let mayor = m
        .roads()
        .find(|r| r.name == "Calle Mayor")
        .map(|r| r.id)
        .ok_or("sin Calle Mayor")?;
    change(&mut m, pb, p(-40.0, 0.0), |n, arms| {
        n.main = Some(mayor);
        for a in arms {
            let r = Junction::arm_ref(a);
            if a.road != mayor {
                n.set_arm(
                    r,
                    ArmSettings {
                        sign: Some(ArmSign::Yield),
                        ..Default::default()
                    },
                );
            }
            n.set_corner(
                r,
                CornerSettings {
                    bump: Some(2.0),
                    ..Default::default()
                },
            );
        }
    })?;
    change(&mut m, pb, p(40.0, 60.0), |n, _| {
        n.kind = Some(NodeKind::Open)
    })?;
    // The T: chamfered corners (Barcelona) of 6 m and the street that ends
    // there set back 18 m.
    change(&mut m, pb, p(0.0, 0.0), |n, arms| {
        for a in arms {
            let r = Junction::arm_ref(a);
            n.set_corner(
                r,
                CornerSettings {
                    radius: Some(6.0),
                    shape: Some(CornerShape::Chamfer),
                    ..Default::default()
                },
            );
        }
        // The Calle del Sauce: the only arm going south.
        if let Some(a) = arms.iter().find(|a| a.dir.y < -0.9) {
            n.set_arm(
                Junction::arm_ref(a),
                ArmSettings {
                    setback: Some(18.0),
                    sign: Some(ArmSign::Stop),
                    ..Default::default()
                },
            );
        }
    })?;
    // The L: a sharp corner inside and no sidewalk outside.
    change(&mut m, pb, p(120.0, 100.0), |n, arms| {
        for a in arms {
            let r = Junction::arm_ref(a);
            n.set_corner(
                r,
                CornerSettings {
                    shape: Some(CornerShape::Sharp),
                    sidewalk: Some(false),
                    ..Default::default()
                },
            );
        }
    })?;
    // The Calle del Sauce ends in a turning circle; the bend and the
    // avenue that narrows.
    change(&mut m, pb, p(0.0, -60.0), |n, _| {
        n.kind = Some(NodeKind::DeadEnd);
        n.end_radius = Some(10.0);
    })?;
    change(&mut m, pb, p(160.0, -40.0), |n, arms| {
        n.kind = Some(NodeKind::Bend);
        for a in arms {
            n.set_corner(
                Junction::arm_ref(a),
                CornerSettings {
                    radius: Some(12.0),
                    ..Default::default()
                },
            );
        }
    })?;
    change(&mut m, pb, p(170.0, 30.0), |n, _| {
        n.kind = Some(NodeKind::Continuous);
    })?;
    // Without rules and with precision (N2): a lane in anarchy across the
    // Olmo and the Pino without crossings, a street parallel to the Calle
    // Real 30 m north, climbing at a uniform grade, and a curve typed by
    // its radius going on from the end of the Calle Estrecha.
    add(&mut m, "Carril libre", p(-80.0, 42.0), p(80.0, 42.0))?;
    let free = m
        .roads()
        .find(|r| r.name == "Carril libre")
        .map(|r| r.id)
        .ok_or("sin carril libre")?;
    m.set_road_free(free, true)?;
    let real = m
        .roads()
        .find(|r| r.name == "Calle Real")
        .map(|r| r.id)
        .ok_or("sin Calle Real")?;
    let north = m.parallel_road(real, 40.0)?;
    m.edit_road(north, |r| r.name = "Calle Alta del Parque".into())?;
    m.road_uniform_grade(north, 0.0, 160.0, Some((0.0, 4.0)))?;
    let arc = archisim_core::arc_stretch(
        p(230.0, 30.0),
        archisim_core::Vec2::new(1.0, 0.0),
        25.0,
        std::f64::consts::FRAC_PI_2,
        2.0,
    );
    let narrow = m
        .roads()
        .find(|r| r.name == "Calle Estrecha")
        .map(|r| r.id)
        .ok_or("sin Calle Estrecha")?;
    m.edit_road(narrow, |r| r.axis.extend(arc.iter().skip(1).copied()))?;
    // Lanes by hand (N3): an avenue of two lanes each way crossed by a
    // street; on the avenue, the lane by the kerb only turns right and the
    // other goes straight on or left, behind a stop line set back 3 m; from
    // the street, a connection by hand; and two painted islands.
    let lane = |b, w| archisim_core::strip_of(b, w);
    use archisim_core::RoadBand as B;
    let four = RoadSection {
        markings: Some(paint),
        ..RoadSection::of(vec![
            lane(B::Sidewalk, 4.0),
            lane(B::Kerb, 0.15),
            lane(B::Parking, 2.2),
            lane(B::Carriageway, 3.25),
            lane(B::Carriageway, 3.25),
            lane(B::Carriageway, 3.25),
            lane(B::Carriageway, 3.25),
            lane(B::Parking, 2.2),
            lane(B::Kerb, 0.15),
            lane(B::Sidewalk, 4.0),
        ])
    };
    add_with(
        &mut m,
        "Avenida de los Carriles",
        vec![p(260.0, 0.0), p(400.0, 0.0)],
        &four,
    )?;
    add(
        &mut m,
        "Calle de la Isleta",
        p(330.0, -60.0),
        p(330.0, 60.0),
    )?;
    let avenue = m
        .roads()
        .find(|r| r.name == "Avenida de los Carriles")
        .map(|r| r.id)
        .ok_or("sin avenida")?;
    change(&mut m, pb, p(330.0, 0.0), |n, arms| {
        for a in arms.iter().filter(|a| a.road == avenue) {
            n.set_arm(
                Junction::arm_ref(a),
                ArmSettings {
                    turns: Some(vec![
                        archisim_core::Turns::RIGHT,
                        archisim_core::Turns(
                            archisim_core::Turns::STRAIGHT.0 | archisim_core::Turns::LEFT.0,
                        ),
                    ]),
                    stop_back: Some(3.0),
                    ..Default::default()
                },
            );
        }
        let north = arms.iter().find(|a| a.road != avenue && a.dir.y > 0.9);
        let east = arms.iter().find(|a| a.road == avenue && a.dir.x > 0.9);
        if let (Some(nb), Some(e)) = (north, east) {
            n.links.push(archisim_core::LaneLink {
                from: Junction::arm_ref(nb),
                lane: 0,
                to: Junction::arm_ref(e),
                to_lane: 1,
            });
        }
        // Given on the plan, kept in the node's own axes.
        let (o, u) = crossroads::node_frame(arms, p(330.0, 0.0));
        let local = |q: Point2| p((q - o).dot(u), (q - o).dot(u.perp_left()));
        n.islands.push(archisim_core::PaintedIsland {
            points: [p(333.5, -3.0), p(338.0, -3.0), p(338.0, -7.5)]
                .into_iter()
                .map(local)
                .collect(),
            paint: archisim_core::IslandPaint::Chevrons,
        });
        n.islands.push(archisim_core::PaintedIsland {
            points: [p(322.0, 3.0), p(326.0, 3.0), p(322.0, 7.0)]
                .into_iter()
                .map(local)
                .collect(),
            paint: archisim_core::IslandPaint::Fill,
        });
    })?;
    // A house in the block between the four crossings.
    let corners = [p(-10.0, 15.0), p(10.0, 15.0), p(10.0, 28.0), p(-10.0, 28.0)];
    for i in 0..4 {
        m.add_wall(NewWall {
            wall_type: lib.bearing_wall,
            level: pb,
            start: corners[i],
            end: corners[(i + 1) % 4],
            reference: ReferenceLine::ExteriorFace,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
    }
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("nudos.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(dir.join("nudos.ifc"), crate::ifc_text(&m, "nudos", None))?;
    let pb = m.levels()[0].id;
    let options = archisim_plan::PlanOptions {
        scale: 500.0,
        ..Default::default()
    };
    std::fs::write(
        dir.join("nudos-planta.svg"),
        archisim_plan::render_plan(&m, pb, &options),
    )?;
    let axo = archisim_core::Axo::default();
    let drawing = archisim_plan::axo::axo_drawing(&m, &axo, Default::default());
    let crop = archisim_plan::axo::axo_extent(&m, &axo, 500.0);
    std::fs::write(
        dir.join("nudos-3d.svg"),
        archisim_plan::axo::render_axo(&drawing, crop, 500.0),
    )?;
    println!(
        "Nudos: {} calles, {} nudos cambiados a mano.",
        m.roads().count(),
        m.nodes().count()
    );
    for j in crossroads::junctions(&m, pb) {
        let Some(k) = &j.key else { continue };
        let names: Vec<String> = k
            .road_ids()
            .filter_map(|r| m.road(r).map(|x| x.name.clone()))
            .collect();
        let mut extra = Vec::new();
        if !j.signs.is_empty() {
            extra.push(match j.signs.len() {
                1 => "1 señal".to_string(),
                n => format!("{n} señales"),
            });
        }
        if !j.bump_kerbs.is_empty() {
            extra.push("orejas".to_string());
        }
        if j.flat && j.kind != NodeKind::DeadEnd {
            extra.push("plano".to_string());
        }
        if !j.quiet.is_empty() {
            extra.push("sin marcas".to_string());
        }
        if !j.arm_paint.is_empty() {
            extra.push("flechas de carril".to_string());
        }
        let hand = j.lanes.iter().filter(|c| c.by_hand).count();
        match hand {
            0 => {}
            1 => extra.push("1 conexión a mano".to_string()),
            _ => extra.push(format!("{hand} conexiones a mano")),
        }
        if let Some(n) = m.node(k).filter(|n| !n.islands.is_empty()) {
            extra.push(match n.islands.len() {
                1 => "1 isleta pintada".to_string(),
                c => format!("{c} isletas pintadas"),
            });
        }
        println!(
            "  {}: {}{}{}",
            names.join(" y "),
            j.kind.label(),
            if m.node(k).is_some() {
                ", cambiado"
            } else {
                ""
            },
            if extra.is_empty() {
                String::new()
            } else {
                format!(" ({})", extra.join(", "))
            }
        );
    }
    for n in m.nodes().filter(|n| n.kind() == NodeKind::Open) {
        let names: Vec<String> = n
            .key
            .road_ids()
            .filter_map(|r| m.road(r).map(|x| x.name.clone()))
            .collect();
        println!("  {}: sin nudo", names.join(" y "));
    }
    println!("  Proyecto: {}", project.display());
    println!("  Planta: {}", dir.join("nudos-planta.svg").display());
    println!("  Axonometría: {}", dir.join("nudos-3d.svg").display());
    println!("  IFC: {}", dir.join("nudos.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kind of node is there, and the nodes' settings come back from
    /// the file and from the IFC.
    #[test]
    fn the_nodes_come_back() {
        let m = build().unwrap();
        let pb = m.levels()[0].id;
        let kinds: Vec<NodeKind> = crossroads::junctions(&m, pb)
            .iter()
            .filter(|j| j.key.is_some())
            .map(|j| j.kind)
            .collect();
        for k in [
            NodeKind::Crossing,
            NodeKind::Raised,
            NodeKind::Shared,
            NodeKind::DeadEnd,
            NodeKind::Bend,
            NodeKind::Continuous,
        ] {
            assert!(kinds.contains(&k), "{k:?}");
        }
        let here = m.nodes().count();
        assert_eq!(here, 10);
        // The lanes by hand (N3): arrows, a connection and two islands.
        let lanes = crossroads::junctions(&m, pb)
            .into_iter()
            .find(|j| j.lanes.iter().any(|c| c.by_hand))
            .expect("the avenue's crossing");
        assert!(!lanes.arm_paint.is_empty());
        let n = m.node(lanes.key.as_ref().unwrap()).unwrap();
        assert_eq!(n.islands.len(), 2);
        // The lane in anarchy crosses the Olmo and the Pino without
        // junctions.
        let free = m.roads().find(|r| r.free).unwrap().id;
        assert!(crossroads::junctions(&m, pb)
            .iter()
            .all(|j| j.arms.iter().all(|a| a.road != free)));
        // The file.
        let text = archisim_file::to_string(&m);
        let back = archisim_file::from_str(&text).unwrap();
        assert_eq!(
            back.nodes().collect::<Vec<_>>(),
            m.nodes().collect::<Vec<_>>()
        );
        // The IFC: the same settings, by the streets brought back.
        let ifc = crate::ifc_text(&m, "nudos", None);
        let imported = archisim_ifc::import_ifc(&ifc).unwrap().model;
        assert_eq!(imported.nodes().count(), here);
        assert_eq!(imported.roads().filter(|r| r.free).count(), 1);
        assert_eq!(back.roads().filter(|r| r.free).count(), 1);
        let kinds_back: Vec<Option<NodeKind>> = imported.nodes().map(|n| n.kind).collect();
        let kinds_here: Vec<Option<NodeKind>> = m.nodes().map(|n| n.kind).collect();
        let mut a = kinds_back.clone();
        let mut b = kinds_here.clone();
        a.sort_by_key(|k| format!("{k:?}"));
        b.sort_by_key(|k| format!("{k:?}"));
        assert_eq!(a, b);
        let islands =
            |m: &Model| -> usize { m.nodes().map(|n| n.islands.len() + n.links.len()).sum() };
        assert_eq!(islands(&imported), islands(&m));
        let ipb = imported.levels()[0].id;
        assert_eq!(
            crossroads::junctions(&imported, ipb).len(),
            crossroads::junctions(&m, pb).len()
        );
    }
}
