//! `archisim gama`: the stylized range of materials (2026-11-02) in a
//! sample board, one piece each (walls standing, roof coverings on a
//! gable, pavings lying), and a little street with a cliff and the sea,
//! to see them in the 3D view and the Maqueta.

use std::path::Path;

use archisim_core::{MassShape, MaterialId, Model, NewMass, Point2, Primitive};
use archisim_library::stylized::{stylized_material, STYLIZED};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn rect(x: f64, y: f64, w: f64, d: f64) -> MassShape {
    MassShape::Prism(vec![
        Point2::new(x, y),
        Point2::new(x + w, y),
        Point2::new(x + w, y + d),
        Point2::new(x, y + d),
    ])
}

/// A gable `w` long (along x) and `d` deep, centred on (x, y).
fn gable(x: f64, y: f64, w: f64, d: f64) -> MassShape {
    MassShape::Sweep {
        path: vec![Point2::new(x - w / 2.0, y), Point2::new(x + w / 2.0, y)],
        width: d,
        profile: Some(vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.5, 1.0),
        ]),
    }
}

struct Builder {
    m: Model,
    level: archisim_core::LevelId,
}

impl Builder {
    fn mat(&mut self, name: &str) -> Res<MaterialId> {
        if let Some(id) = stylized_material(&mut self.m, name) {
            return Ok(id);
        }
        self.m
            .materials()
            .find(|x| x.name == name)
            .map(|x| x.id)
            .ok_or_else(|| format!("falta el material «{name}»").into())
    }

    fn mass(&mut self, name: &str, shape: MassShape, base: f64, h: f64, mat: &str) -> Res {
        let material = self.mat(mat)?;
        self.m.add_mass(NewMass {
            name: name.into(),
            level: self.level,
            shape,
            base_offset: base,
            height: h,
            floor_height: None,
            material,
        })?;
        Ok(())
    }
}

pub fn build() -> Res<Model> {
    let m = archisim_library::new_project("Gama estilizada de ArchiSim");
    let level = m.levels()[0].id;
    let mut b = Builder { m, level };

    // The board: one row a group, the pieces 3.5 m apart.
    let groups = ["Fachadas", "Cubiertas", "Suelos", "Pavimentos", "Pinturas"];
    b.mass(
        "Tablero",
        rect(-20.0, -2.0, 40.0, 30.0),
        -0.2,
        0.2,
        "Hormigón suave",
    )?;
    for (gi, group) in groups.iter().enumerate() {
        let items: Vec<_> = STYLIZED.iter().filter(|s| s.group == *group).collect();
        let y = 24.0 - gi as f64 * 5.5;
        for (i, s) in items.iter().enumerate() {
            let x = (i as f64 - (items.len() as f64 - 1.0) / 2.0) * 3.5;
            match *group {
                "Fachadas" => b.mass(s.name, rect(x - 1.3, y, 2.6, 0.5), 0.0, 2.6, s.name)?,
                "Cubiertas" => b.mass(s.name, gable(x, y, 2.6, 2.6), 0.0, 1.3, s.name)?,
                "Pinturas" => b.mass(s.name, rect(x - 0.9, y - 0.9, 1.8, 1.8), 0.0, 1.8, s.name)?,
                _ => b.mass(
                    s.name,
                    rect(x - 1.35, y - 1.35, 2.7, 2.7),
                    0.0,
                    0.12,
                    s.name,
                )?,
            }
        }
    }

    // A little street in front: a lawn, a pavement of setts, houses, a
    // round tower, and below the road a cliff of strata down to the sea.
    let y0 = -16.0;
    b.mass("Césped", rect(-24.0, y0, 48.0, 10.0), -0.05, 0.05, "Césped")?;
    b.mass(
        "Acera",
        rect(-24.0, y0 - 3.0, 48.0, 3.0),
        -0.05,
        0.12,
        "Adoquín",
    )?;
    b.mass(
        "Calzada",
        rect(-24.0, y0 - 10.0, 48.0, 7.0),
        -0.15,
        0.1,
        "Asfalto",
    )?;
    b.mass(
        "Acantilado",
        rect(-24.0, y0 - 11.0, 48.0, 1.0),
        -7.0,
        6.85,
        "Roca en estratos",
    )?;
    b.mass(
        "Fondo del mar",
        rect(-60.0, y0 - 60.0, 120.0, 49.0),
        -9.0,
        1.5,
        "Grava",
    )?;
    b.mass(
        "Mar",
        rect(-60.0, y0 - 60.0, 120.0, 49.0),
        -6.6,
        0.05,
        "Agua de mar",
    )?;
    let houses = [
        (
            "Casa de ladrillo",
            "Ladrillo amarillo",
            "Tejuela de pizarra",
            -18.0,
            7.0,
            6.0,
        ),
        (
            "Casa de tablas",
            "Tablas solapadas azul",
            "Tejuela de pizarra",
            -9.5,
            6.5,
            6.2,
        ),
        (
            "Casa de sillería",
            "Sillería arenisca",
            "Tejuela verde",
            -1.5,
            6.0,
            8.8,
        ),
        (
            "Casa de estuco",
            "Estuco amarillo",
            "Escamas de pizarra",
            5.5,
            5.5,
            7.2,
        ),
        ("Casa encalada", "Encalado", "Teja curva", 19.0, 7.0, 6.0),
    ];
    for (name, wall, roof, x, w, h) in houses {
        b.mass(name, rect(x - w / 2.0, y0 + 2.0, w, 7.0), 0.0, h, wall)?;
        b.mass(
            &format!("{name}, cubierta"),
            gable(x, y0 + 5.5, w + 0.6, 7.6),
            h,
            2.6,
            roof,
        )?;
    }
    b.mass(
        "Torre",
        MassShape::Primitive(Primitive::Cylinder {
            center: Point2::new(12.0, y0 + 5.0),
            radius: 2.6,
        }),
        0.0,
        12.0,
        "Mampostería",
    )?;
    b.mass(
        "Torre, chapitel",
        MassShape::Primitive(Primitive::Cone {
            center: Point2::new(12.0, y0 + 5.0),
            radius: 3.1,
            top_radius: 0.0,
        }),
        12.0,
        5.0,
        "Cobre con pátina",
    )?;
    // A pool in the garden of the brick house.
    let (px, py, pl, pw, t) = (-21.5, y0 + 0.2, 7.0, 1.8, 0.15);
    b.mass("Vaso, fondo", rect(px, py, pl, pw), 0.0, 0.06, "Gresite")?;
    b.mass("Vaso, delante", rect(px, py, pl, t), 0.0, 0.5, "Gresite")?;
    b.mass(
        "Vaso, detrás",
        rect(px, py + pw - t, pl, t),
        0.0,
        0.5,
        "Gresite",
    )?;
    b.mass(
        "Vaso, izquierda",
        rect(px, py + t, t, pw - 2.0 * t),
        0.0,
        0.5,
        "Gresite",
    )?;
    b.mass(
        "Vaso, derecha",
        rect(px + pl - t, py + t, t, pw - 2.0 * t),
        0.0,
        0.5,
        "Gresite",
    )?;
    b.mass(
        "Agua de la piscina",
        rect(px + t, py + t, pl - 2.0 * t, pw - 2.0 * t),
        0.36,
        0.06,
        "Agua de piscina",
    )?;
    Ok(b.m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("gama.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    let back = archisim_file::open(&project)?;
    if back != m {
        return Err("el proyecto de la gama no vuelve igual al abrirlo".into());
    }
    println!(
        "Escrito {} ({} materiales de la gama estilizada)",
        project.display(),
        STYLIZED.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_range_builds_and_opens_again() {
        let m = super::build().unwrap();
        let named = |n: &str| m.materials().any(|x| x.name == n);
        for s in archisim_library::stylized::STYLIZED {
            assert!(named(s.name), "{}", s.name);
        }
    }
}
