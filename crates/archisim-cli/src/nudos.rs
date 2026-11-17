//! `archisim nudos`: the nodes of the streets changed by hand (N1,
//! `docs/TERRITORIO.md` §2) — a grid of four streets with one crossing of
//! each kind (crossing, raised, shared surface and no node), a T with a
//! chamfered corner and longer setbacks, and an L with a sharp corner and
//! no sidewalk at one corner. In plan, in 3D and in the IFC (whose
//! `ArchiSim_Road` sets keep the nodes' settings, read back on import).

use std::path::Path;

use archisim_core::{
    ArmSettings, CornerSettings, CornerShape, LevelId, MaterialId, Model, NewRoad, NewWall,
    NodeKind, NodeSettings, Point2, ReferenceLine, RoadSection, WallTop,
};
use archisim_geom::crossroads::{self, Junction};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn material(m: &Model, category: &str) -> Option<MaterialId> {
    m.materials().find(|x| x.category == category).map(|x| x.id)
}

/// The node at `p` changed by `f`.
fn change(
    m: &mut Model,
    level: LevelId,
    p: Point2,
    f: impl FnOnce(&mut NodeSettings, &Junction),
) -> Res {
    let j = crossroads::node_at(m, level, p, 5.0).ok_or("no hay nudo")?;
    let key = j.key.clone().ok_or("nudo sin clave")?;
    let mut n = m
        .node(&key)
        .cloned()
        .unwrap_or_else(|| NodeSettings::new(key));
    f(&mut n, &j);
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
    let add = |m: &mut Model, name: &str, a: Point2, b: Point2| -> Res {
        m.add_road(NewRoad {
            name: name.into(),
            level: pb,
            axis: vec![a, b],
            section: RoadSection::default(),
            top_offset: 0.0,
            profile: Vec::new(),
            changes: Vec::new(),
            pavement: asphalt,
            kerb_material: stone,
            sidewalk_material: paving,
        })?;
        Ok(())
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
    // One crossing of each kind.
    change(&mut m, pb, p(40.0, 0.0), |n, _| {
        n.kind = Some(NodeKind::Raised)
    })?;
    change(&mut m, pb, p(-40.0, 60.0), |n, _| {
        n.kind = Some(NodeKind::Shared)
    })?;
    change(&mut m, pb, p(40.0, 60.0), |n, _| {
        n.kind = Some(NodeKind::Open)
    })?;
    // The T: chamfered corners (Barcelona) of 6 m and the street that ends
    // there set back 18 m.
    change(&mut m, pb, p(0.0, 0.0), |n, j| {
        for a in &j.arms {
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
        if let Some(a) = j.arms.iter().find(|a| a.dir.y < -0.9) {
            n.set_arm(
                Junction::arm_ref(a),
                ArmSettings {
                    setback: Some(18.0),
                },
            );
        }
    })?;
    // The L: a sharp corner inside and no sidewalk outside.
    change(&mut m, pb, p(120.0, 100.0), |n, j| {
        for a in &j.arms {
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
        println!(
            "  {} ({} calles): {}{}",
            names.join(" y "),
            j.arms.len(),
            j.kind.label(),
            if m.node(k).is_some() {
                ", cambiado"
            } else {
                ""
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
        for k in [NodeKind::Crossing, NodeKind::Raised, NodeKind::Shared] {
            assert!(kinds.contains(&k), "{k:?}");
        }
        assert_eq!(m.nodes().count(), 5);
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
        assert_eq!(imported.nodes().count(), 5);
        let kinds_back: Vec<Option<NodeKind>> = imported.nodes().map(|n| n.kind).collect();
        let kinds_here: Vec<Option<NodeKind>> = m.nodes().map(|n| n.kind).collect();
        let mut a = kinds_back.clone();
        let mut b = kinds_here.clone();
        a.sort_by_key(|k| format!("{k:?}"));
        b.sort_by_key(|k| format!("{k:?}"));
        assert_eq!(a, b);
        let ipb = imported.levels()[0].id;
        assert_eq!(
            crossroads::junctions(&imported, ipb).len(),
            crossroads::junctions(&m, pb).len()
        );
    }
}
