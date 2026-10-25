//! `archisim cocina`: a modular kitchen (concepto de interfaz del
//! 24/10/2026): a run of base units against a wall — fridge column,
//! drawers, sink, dishwasher, oven, hob, bottle rack — their worktops
//! meeting as one, wall units over them and a hood over the hob; and the
//! same run with framed, grooved and glazed fronts, to compare. Then (C2)
//! a kitchen in an L in a corner, an island of modules back to back and a
//! corner sofa: each run of modules under one worktop.

use std::path::Path;

use archisim_core::{Model, NewObject, NewWall, Point2, ReferenceLine, Value, WallTop};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// The run, from the left: the library's type and its width.
const RUN: &[&str] = &[
    "Columna frigorífico 60",
    "Bajo cajonera 60",
    "Bajo fregadero 80",
    "Bajo lavavajillas 60",
    "Bajo horno 60",
    "Bajo placa 90",
    "Botellero 15",
    "Bajo cajonera 40",
];

/// Wall units over the base units after the column (by position in RUN).
const HIGH: &[(usize, &str)] = &[
    (1, "Alto con puerta 40"),
    (2, "Alto vitrina 80"),
    (3, "Alto con puerta 40"),
    (4, "Alto con puerta 40"),
    (6, "Alto con puerta 40"),
];

pub fn build() -> Res<Model> {
    let mut m = archisim_library::new_project("Cocina por módulos");
    let level = m.levels()[0].id;
    let wall = m
        .wall_types()
        .find(|t| t.name.contains("Tabique"))
        .map(|t| t.id);
    let wall = wall
        .or_else(|| m.wall_types().next().map(|t| t.id))
        .ok_or("sin muros")?;
    let styles = ["Liso", "Marco", "Ranurado", "Cristal"];
    for (row, style) in styles.iter().enumerate() {
        let y0 = row as f64 * -3.5;
        // The wall behind, its inner face on the line of the backs.
        m.add_wall(NewWall {
            wall_type: wall,
            level,
            // Its outside (on its left, drawn left to right) behind: the
            // kitchen looks south, to the usual view from the south-west.
            start: Point2::new(-0.2, y0),
            end: Point2::new(5.0, y0),
            reference: ReferenceLine::InteriorFace,
            height: 2.6,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
        let mut x = 0.0;
        let mut centres = Vec::new();
        for name in RUN {
            let t = archisim_library::add_object_entry(&mut m, name)?;
            let (lo, hi) = m.object_type_box_3d(t).ok_or("sin caja")?;
            let w = hi[0] - lo[0];
            let id = m.add_object(NewObject {
                level,
                object_type: t,
                position: Point2::new(x + w / 2.0, y0),
                rotation: std::f64::consts::PI,
                base_offset: 0.0,
            })?;
            centres.push((x + w / 2.0, id));
            x += w;
        }
        for (i, name) in HIGH {
            let t = archisim_library::add_object_entry(&mut m, name)?;
            let (cx, _) = centres[*i];
            m.add_object(NewObject {
                level,
                object_type: t,
                position: Point2::new(cx, y0),
                rotation: std::f64::consts::PI,
                base_offset: 0.0,
            })?;
        }
        let hood = archisim_library::add_object_entry(&mut m, "Campana decorativa 60")?;
        m.add_object(NewObject {
            level,
            object_type: hood,
            position: Point2::new(centres[5].0, y0),
            rotation: std::f64::consts::PI,
            base_offset: 0.0,
        })?;
        // This row's fronts: each module its own type in the style.
        if row > 0 {
            let ids: Vec<_> = m
                .objects()
                .filter(|o| (o.position.y - y0).abs() < 1e-6)
                .map(|o| (o.id, o.object_type))
                .collect();
            for (id, t) in ids {
                let Some(ty) = m.object_type(t).cloned() else {
                    continue;
                };
                let has = m
                    .family(ty.family)
                    .is_some_and(|f| f.def().params.iter().any(|p| p.name == "front_style"));
                if !has {
                    continue;
                }
                let mut values = ty.values.clone();
                values.insert("front_style".into(), Value::Text((*style).into()));
                let name = format!("{} · {}", ty.name, style);
                let found = m.object_types().find(|x| x.name == name).map(|x| x.id);
                let nt = match found {
                    Some(x) => x,
                    None => m.add_object_type(name, ty.family, values)?,
                };
                m.edit_object(id, |o| o.object_type = nt)?;
            }
        }
    }
    corner_and_island(&mut m, wall)?;
    Ok(m)
}

/// A kitchen in an L against two walls, an island back to back in front
/// of it, and a corner sofa beside them.
fn corner_and_island(m: &mut Model, wall: archisim_core::WallTypeId) -> Res {
    let level = m.levels()[0].id;
    let y0 = -16.0;
    let add_wall = |m: &mut Model, a: Point2, b: Point2| -> Res {
        m.add_wall(NewWall {
            wall_type: wall,
            level,
            start: a,
            end: b,
            reference: ReferenceLine::InteriorFace,
            height: 2.6,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
        Ok(())
    };
    add_wall(m, Point2::new(-0.2, y0), Point2::new(3.0, y0))?;
    add_wall(m, Point2::new(3.0, y0), Point2::new(3.0, y0 - 3.5))?;
    let put = |m: &mut Model, name: &str, at: Point2, rotation: f64| -> Res {
        let t = archisim_library::add_object_entry(m, name)?;
        m.add_object(NewObject {
            level,
            object_type: t,
            position: at,
            rotation,
            base_offset: 0.0,
        })?;
        Ok(())
    };
    // A type of the library with some values changed (a copy of it,
    // named after what changes).
    let variant = |m: &mut Model,
                   name: &str,
                   what: &str,
                   set: &[(&str, f64)]|
     -> Result<archisim_core::ObjectTypeId, Box<dyn std::error::Error>> {
        let base = archisim_library::add_object_entry(m, name)?;
        let t = m.object_type(base).ok_or("type")?.clone();
        let mut values = t.values.clone();
        for (k, v) in set {
            values.insert((*k).into(), archisim_core::family::Value::Number(*v));
        }
        Ok(m.add_object_type(format!("{name} {what}"), t.family, values)?)
    };
    let place = |m: &mut Model, t: archisim_core::ObjectTypeId, at: Point2, rotation: f64| -> Res {
        m.add_object(NewObject {
            level,
            object_type: t,
            position: at,
            rotation,
            base_offset: 0.0,
        })?;
        Ok(())
    };
    let pi = std::f64::consts::PI;
    let quarter = std::f64::consts::FRAC_PI_2;
    // Along the back wall, looking south, to the corner (x = 3); with a
    // 10 cm upstand against the walls.
    let upstand = [("upstand", 0.1)];
    for (name, x) in [
        ("Bajo cajonera 60", 0.7),
        ("Bajo fregadero 80", 1.4),
        ("Bajo lavavajillas 60", 2.1),
        ("Bajo cajonera 60", 2.7),
    ] {
        let t = variant(m, name, "con copete", &upstand)?;
        place(m, t, Point2::new(x, y0), pi)?;
    }
    // Down the side wall, looking west: the first against the corner
    // module's front.
    let t = variant(m, "Bajo cajonera 60", "con copete", &upstand)?;
    place(m, t, Point2::new(3.0, y0 - 0.9), quarter)?;
    let t = variant(m, "Bajo placa 90", "con copete", &upstand)?;
    place(m, t, Point2::new(3.0, y0 - 1.65), quarter)?;
    // The island: three modules each way, backs together; on the south
    // side the worktop overhangs 30 cm, a bar to sit at.
    let bar = variant(m, "Bajo cajonera 60", "con vuelo", &[("overhang", 0.3)])?;
    let plain = archisim_library::add_object_entry(m, "Bajo cajonera 60")?;
    for x in [0.6, 1.2, 1.8] {
        place(m, bar, Point2::new(x, y0 - 2.6), pi)?;
        place(m, plain, Point2::new(x, y0 - 2.6), 0.0)?;
    }
    put(m, "Sofá rinconera 260×170", Point2::new(6.5, y0 - 0.5), pi)?;
    Ok(())
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("cocina.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(dir.join("cocina.ifc"), crate::ifc_text(&m, "cocina", None))?;
    let axo = archisim_core::Axo::default();
    let drawing = archisim_plan::axo::axo_drawing(&m, &axo, Default::default());
    let crop = archisim_plan::axo::axo_extent(&m, &axo, 30.0);
    std::fs::write(
        dir.join("cocina-3d.svg"),
        archisim_plan::axo::render_axo(&drawing, crop, 30.0),
    )?;
    let options = archisim_plan::PlanOptions {
        scale: 50.0,
        ..Default::default()
    };
    std::fs::write(
        dir.join("cocina-planta.svg"),
        archisim_plan::render_plan(&m, m.levels()[0].id, &options),
    )?;
    println!(
        "Cocina por módulos: {} piezas en cuatro filas (frentes liso, con marco, ranurado y con cristal), una cocina en L, una isla y un sofá rinconera.",
        m.objects().count()
    );
    println!("  Proyecto: {}", project.display());
    println!("  Axonometría: {}", dir.join("cocina-3d.svg").display());
    println!("  IFC: {}", dir.join("cocina.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_modules_stand_side_by_side_with_their_3d() {
        let m = super::build().unwrap();
        assert!(m.objects().all(|o| m.type_has_prop(o.object_type)));
        // Four rows of eight base units, five wall units and a hood; the
        // L (six), the island (six) and the sofa.
        assert_eq!(m.objects().count(), 4 * (8 + 5 + 1) + 6 + 6 + 1);
        let solids = archisim_geom::solid::model_solids(&m);
        assert!(solids.len() > 50);
        // A worktop for each row of base units (the column at its end
        // is not under it), one for the L and one for the island.
        let runs = m.worktop_runs();
        assert_eq!(runs.len(), 4 + 2, "{}", runs.len());
        assert!(runs
            .iter()
            .any(|r| r.members.len() == 6 && r.sinks.len() == 1));
        assert!(runs
            .iter()
            .any(|r| r.members.len() == 6 && r.sinks.is_empty()));
        // Measured: worktops in metres along the fronts (each row 4,05 m,
        // the L 4,10 m, the island 3,60 m) and the modules by units.
        let chapters = archisim_plan::schedules::measurements(&m);
        let (_, title, items) = chapters.iter().find(|c| c.0 == "10").expect("chapter 10");
        assert_eq!(*title, "Mobiliario de cocina");
        let tops: f64 = items
            .iter()
            .filter(|i| i.key.starts_with("encimera:"))
            .map(|i| i.total())
            .sum();
        assert!((tops - (4.0 * 4.05 + 4.1 + 3.6)).abs() < 1e-6, "{tops}");
        // The L's upstand, all along its backs; the island's bar.
        let upstands: f64 = items
            .iter()
            .filter(|i| i.key.starts_with("copete:"))
            .map(|i| i.total())
            .sum();
        assert!((upstands - 4.1).abs() < 1e-6, "{upstands}");
        let island = runs
            .iter()
            .find(|r| r.members.len() == 6 && r.sinks.is_empty())
            .unwrap();
        assert!(
            (island.area - 1.8 * (0.9 + 0.6)).abs() < 1e-6,
            "{}",
            island.area
        );
        let units: f64 = items
            .iter()
            .filter(|i| i.key.starts_with("mueble:"))
            .map(|i| i.total())
            .sum();
        // Every object of a kitchen family: all but the sofa.
        assert_eq!(units as usize, m.objects().count() - 1);
    }
}
