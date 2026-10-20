//! A sample board of the template's doors and windows ("muestrario"): one
//! long wall with every type in it, to see them in plan, 3D and IFC.

use std::path::Path;

use archisim_core::{Hand, Model, NewOpening, NewWall, Point2, ReferenceLine, WallSide, WallTop};

/// Gap between two openings, and at the ends of the wall.
const GAP: f64 = 0.6;

/// The template with every door and window type in one wall.
pub fn build() -> Result<Model, Box<dyn std::error::Error>> {
    let mut m = archisim_library::new_project("Muestrario de puertas y ventanas");
    let level = m.levels()[0].id;
    let wall_type = m
        .wall_types()
        .find(|t| t.name.starts_with("Fachada"))
        .map(|t| t.id)
        .ok_or("la plantilla no tiene fachada")?;
    let types: Vec<(archisim_core::OpeningTypeId, f64, f64, bool)> = m
        .opening_types()
        .map(|t| (t.id, t.width, t.height, t.is_door()))
        .collect();
    let length: f64 = types.iter().map(|t| t.1 + GAP).sum::<f64>() + GAP;
    let tallest = types.iter().map(|t| t.2).fold(0.0, f64::max);
    let wall = m.add_wall(NewWall {
        wall_type,
        level,
        start: Point2::new(0.0, 0.0),
        end: Point2::new(length, 0.0),
        bulge: 0.0,
        reference: ReferenceLine::Center,
        height: (tallest + 1.4).max(3.0),
        base_offset: 0.0,
        top: WallTop::Unconnected,
    })?;
    let mut x = GAP;
    for (id, width, height, door) in types {
        // Windows at 90 cm, those that reach the floor (balconeras) on it.
        let sill = if door || height >= 2.0 { 0.0 } else { 0.9 };
        m.add_opening(NewOpening {
            opening_type: id,
            wall,
            along: x + width / 2.0,
            sill,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })?;
        x += width + GAP;
    }
    Ok(m)
}

/// Writes the board as a project, an IFC and a 1:50 plan.
pub fn write(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let m = build()?;
    let project = dir.join("muestrario.archisim");
    archisim_file::save(&m, &project)?;
    println!("  Proyecto: {}", project.display());
    let ifc = dir.join("muestrario.ifc");
    std::fs::write(&ifc, crate::ifc_text(&m, "muestrario", None))?;
    println!("  IFC 4.3: {}", ifc.display());
    let level = m.levels()[0].id;
    let svg = dir.join("muestrario.svg");
    std::fs::write(
        &svg,
        archisim_plan::render_plan(&m, level, &archisim_plan::PlanOptions::default()),
    )?;
    println!("  Planta 1:50: {}", svg.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Every door and window of the template fits in the board.
    #[test]
    fn every_type_is_on_the_board() {
        let m = super::build().unwrap();
        assert_eq!(m.openings().count(), m.opening_types().count());
    }
}
