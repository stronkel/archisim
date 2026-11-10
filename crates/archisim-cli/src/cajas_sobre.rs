//! `archisim cajas-sobre`: the boxes of a modular house drawn in another
//! program and imported (M12, `docs/MODULAR.md`). Each floor slab of a
//! box's size becomes a box (grown to the module when it fits), its sides
//! are read from the walls along the joints, and each box can be written as
//! a box type of the catalogue.

use std::fmt::Write as _;
use std::path::Path;

use archisim_core::{CatalogueEntry, Model, SideState};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Puts the boxes in; returns what it says.
pub fn place(m: &mut Model, module: (f64, f64)) -> Res<(Vec<archisim_core::BoxId>, String)> {
    let floors = m.box_floor_slabs();
    if floors.is_empty() {
        return Err("no hay suelos del tamaño de una caja (de 4 a 9,5 m por 2,4 a 3,8 m)".into());
    }
    archisim_library::frame_material(m, archisim_core::FrameMaterial::Steel);
    archisim_library::joint_materials(m);
    let boxes = m.boxes_on_floors(&floors, Some(module))?;
    m.box_sides_from_walls(&boxes)?;
    let mut t = String::new();
    let _ = writeln!(
        t,
        "{} cajas sobre {} suelos (módulo {:.2} × {:.2} m).",
        boxes.len(),
        floors.len(),
        module.0,
        module.1
    );
    let letter = |s: SideState| match s {
        SideState::Free => "libre",
        SideState::Wall => "muro",
        SideState::Open => "abierto",
    };
    for id in &boxes {
        let b = m.modular_box(*id).unwrap();
        let held = m.box_members(*id);
        let _ = writeln!(
            t,
            "  {}: {:.2} × {:.2} m · lados A {}, B {}, C {}, D {} · {} muros, {} puertas y ventanas, {} objetos",
            b.mark,
            b.length,
            b.width,
            letter(b.sides[0]),
            letter(b.sides[1]),
            letter(b.sides[2]),
            letter(b.sides[3]),
            held.walls.len(),
            held.openings.len(),
            held.objects.len()
        );
        for issue in m.box_issues(*id) {
            let _ = writeln!(
                t,
                "      {} {}",
                if issue.is_error() { "✗" } else { "·" },
                issue.text(m)
            );
        }
    }
    Ok((boxes, t))
}

/// Makes the boxes over `input` and writes the project to `output`; with
/// `types`, one box type per box into that folder (codes `<prefix>-nn`).
pub fn run(
    input: &Path,
    output: &Path,
    module: (f64, f64),
    types: Option<(&Path, &str)>,
) -> Res<String> {
    let mut m = archisim_file::open(input)?;
    let (boxes, mut text) = place(&mut m, module)?;
    if let Some(dir) = output.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    archisim_file::save(&m, output)?;
    let _ = writeln!(text, "\nProyecto con las cajas: {}", output.display());
    if let Some((dir, prefix)) = types {
        std::fs::create_dir_all(dir)?;
        for (i, id) in boxes.iter().enumerate() {
            let b = m.modular_box(*id).unwrap();
            let code = format!("{prefix}-{:02}", i + 1);
            let name = format!("Caja {} · {}", b.mark, prefix);
            let mut t = m.box_type_model(*id, CatalogueEntry::new(code.clone(), name.clone()))?;
            t.project_name = name;
            let path = dir.join(format!("{code}.{}", archisim_file::EXTENSION));
            archisim_file::save(&t, &path)?;
            let _ = writeln!(text, "Caja tipo {code} (en diseño): {}", path.display());
        }
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_boxes_of_the_example_are_found_again() {
        // The example's boxes taken away: their floors make them again.
        let mut m = crate::cajas::build().unwrap();
        let before = m.boxes().count();
        let ids: Vec<_> = m.boxes().map(|b| b.id).collect();
        for id in ids {
            m.remove_box(id).unwrap();
        }
        let (boxes, text) = place(
            &mut m,
            (archisim_core::BOX_LENGTH, archisim_core::BOX_WIDTH),
        )
        .unwrap();
        assert_eq!(boxes.len(), before - 2, "the ground floor's: {text}");
        assert!(text.contains("abierto") && text.contains("muro"), "{text}");
    }
}
