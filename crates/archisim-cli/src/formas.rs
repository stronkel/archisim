//! `archisim formas`: the forms of the concept phase side by side, to see
//! them in plan, in the 3D view and in an axonometry: a block with its
//! edges rounded and a cylinder cut out of it, a sphere joined to a box,
//! a cone and a frustum, a vase turned from its profile, what two volumes
//! have in common, the lines where two volumes cross, and faces painted
//! with the visualisation materials.

use std::path::Path;

use archisim_core::{
    EdgeFinish, EdgeKind, FaceRef, MassId, MassShape, Model, NewMass, Point2, Primitive,
};
use archisim_geom::csg::Op;
use archisim_geom::forms::{combine_masses, cross_masses};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn rect(x: f64, y: f64, w: f64, d: f64) -> MassShape {
    MassShape::Prism(vec![
        Point2::new(x, y),
        Point2::new(x + w, y),
        Point2::new(x + w, y + d),
        Point2::new(x, y + d),
    ])
}

pub fn build() -> Res<Model> {
    let mut m = archisim_library::new_project("Formas de ArchiSim");
    let level = m.levels()[0].id;
    let material = |m: &Model, name: &str| {
        m.materials()
            .find(|x| x.name == name)
            .map(|x| x.id)
            .ok_or_else(|| format!("falta el material «{name}»"))
    };
    let concept = material(&m, "Volumen de concepto")?;
    let brick = material(&m, "Ladrillo visto")?;
    let stone = material(&m, "Piedra")?;
    let wood = material(&m, "Tarima de madera")?;
    let grass = material(&m, "Césped")?;
    let concrete = material(&m, "Hormigón armado")?;
    let add = |m: &mut Model, name: &str, shape: MassShape, h: f64, mat| -> Res<MassId> {
        Ok(m.add_mass(NewMass {
            name: name.into(),
            level,
            shape,
            base_offset: 0.0,
            height: h,
            floor_height: None,
            material: mat,
        })?)
    };
    // A lawn under everything.
    add(&mut m, "Césped", rect(-4.0, -12.0, 52.0, 26.0), 0.0, grass)?;

    // A brick block with its corners and top rounded, a concrete cylinder
    // cut out of one corner.
    let block = add(&mut m, "Bloque", rect(0.0, 0.0, 10.0, 8.0), 6.0, brick)?;
    m.edit_mass(block, |x| {
        x.finish = Some(EdgeFinish {
            kind: EdgeKind::Round,
            size: 0.6,
            vertical: true,
            top: true,
            bottom: false,
        })
    })?;
    let hole = add(
        &mut m,
        "Patio",
        MassShape::Primitive(Primitive::Cylinder {
            center: Point2::new(10.0, 8.0),
            radius: 3.0,
        }),
        8.0,
        concrete,
    )?;
    combine_masses(&mut m, block, &[hole], Op::Subtract)?;

    // A box with a sphere joined on top, the sphere in stone.
    let base = add(&mut m, "Peana", rect(14.0, 0.0, 6.0, 6.0), 3.0, concept)?;
    m.edit_mass(base, |x| {
        x.finish = Some(EdgeFinish {
            kind: EdgeKind::Chamfer,
            size: 0.4,
            vertical: true,
            top: true,
            bottom: false,
        })
    })?;
    let ball = add(
        &mut m,
        "Bola",
        MassShape::Primitive(Primitive::Sphere {
            center: Point2::new(17.0, 3.0),
            radius: 2.5,
        }),
        5.0,
        stone,
    )?;
    m.edit_mass(ball, |x| x.base_offset = 1.5)?;
    combine_masses(&mut m, base, &[ball], Op::Union)?;

    // A cone and a frustum.
    add(
        &mut m,
        "Cono",
        MassShape::Primitive(Primitive::Cone {
            center: Point2::new(25.0, 3.0),
            radius: 2.5,
            top_radius: 0.0,
        }),
        7.0,
        concept,
    )?;
    let frustum = add(
        &mut m,
        "Tronco de cono",
        MassShape::Primitive(Primitive::Cone {
            center: Point2::new(25.0, -6.0),
            radius: 2.5,
            top_radius: 1.2,
        }),
        4.0,
        concrete,
    )?;
    m.edit_mass(frustum, |x| {
        x.finish = Some(EdgeFinish {
            kind: EdgeKind::Round,
            size: 0.3,
            vertical: false,
            top: true,
            bottom: false,
        })
    })?;

    // A vase turned from its half-section, its belly in wood.
    let vase = add(
        &mut m,
        "Jarrón",
        MassShape::Primitive(Primitive::Revolution {
            center: Point2::new(33.0, 3.0),
            profile: vec![
                Point2::new(1.2, 0.0),
                Point2::new(2.6, 2.0),
                Point2::new(1.0, 4.5),
                Point2::new(1.6, 5.5),
            ],
        }),
        5.5,
        stone,
    )?;
    m.set_face_paint(vase.0, FaceRef::Side(0), Some(wood))?;

    // What a cylinder and a box have in common.
    let a = add(&mut m, "Caja", rect(38.0, 0.0, 6.0, 6.0), 4.0, concept)?;
    let b = add(
        &mut m,
        "Cilindro",
        MassShape::Primitive(Primitive::Cylinder {
            center: Point2::new(41.0, 3.0),
            radius: 3.6,
        }),
        5.0,
        wood,
    )?;
    combine_masses(&mut m, a, &[b], Op::Intersect)?;

    // Two volumes that cross, with the lines where their faces meet.
    let c = add(&mut m, "Torre", rect(2.0, -9.0, 4.0, 4.0), 9.0, concept)?;
    let d = add(&mut m, "Puente", rect(0.0, -8.0, 14.0, 2.0), 3.0, concept)?;
    m.edit_mass(d, |x| x.base_offset = 4.0)?;
    cross_masses(&mut m, &[c, d])?;
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("formas.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    // Opened again, the same.
    let back = archisim_file::open(&project)?;
    if back != m {
        return Err("el proyecto de formas no vuelve igual al abrirlo".into());
    }
    let level = m.levels()[0].id;
    let options = archisim_plan::PlanOptions {
        scale: 200.0,
        ..Default::default()
    };
    let plan = archisim_plan::render_plan(&m, level, &options);
    std::fs::write(dir.join("formas-planta.svg"), plan)?;
    let axo = archisim_core::Axo::default();
    let drawing = archisim_plan::axo::axo_drawing(&m, &axo, Default::default());
    let crop = archisim_plan::axo::axo_extent(&m, &axo, 200.0);
    std::fs::write(
        dir.join("formas-3d.svg"),
        archisim_plan::axo::render_axo(&drawing, crop, 200.0),
    )?;
    std::fs::write(dir.join("formas.ifc"), crate::ifc_text(&m, "formas", None))?;
    println!(
        "Formas: {} volúmenes, {} aristas sueltas, {} caras pintadas.",
        m.masses().count(),
        m.edges().count(),
        m.face_paints().count()
    );
    println!("  Proyecto: {}", project.display());
    println!("  Planta: {}", dir.join("formas-planta.svg").display());
    println!("  Axonometría: {}", dir.join("formas-3d.svg").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_form_is_built() {
        let m = super::build().unwrap();
        assert_eq!(m.masses().count(), 9);
        assert!(m.edges().count() >= 1);
        let solids = archisim_geom::solid::model_solids(&m);
        assert!(solids.len() >= 10);
        let text = archisim_file::to_string(&m);
        assert_eq!(archisim_file::from_str(&text).unwrap(), m);
    }
}
