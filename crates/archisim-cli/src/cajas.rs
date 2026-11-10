//! `archisim cajas`: a house of modular boxes in L, as the system's
//! (`docs/MODULAR.md`): six boxes of 6,20 × 3,60 m side by side and two
//! more on the ends of the last two, 2 cm apart, and two on a first floor
//! over the first two (lifted from six points). Each box is filled with
//! its envelope (steel frame façade, floor on its chassis, roof and false
//! ceiling); the joints carry one wall, except the one of the living room,
//! open on both sides. Writes the project, its IFC (each box an
//! `IfcElementAssembly` with its parts and its chassis), the plans and a
//! list of the boxes with what they hold, their notes, their weight and
//! lifting, the order they are craned in and their cut lists.

use std::fmt::Write as _;
use std::path::Path;

use archisim_core::{
    BoxFill, CutPattern, FrameMaterial, Framing, Hand, Layer, LayerFunction, Model, NewBox,
    NewOpening, Point2, SideState, SlabFunction, WallFunction, WallSide, BOX_BOTTOM, BOX_HEIGHT,
    BOX_WIDTH, SIDE_NAMES,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// The joint between two boxes.
const JOINT: f64 = 0.02;

fn material(m: &mut Model, name: &str, class: &str, rgb: [u8; 3]) -> archisim_core::MaterialId {
    let found = m.materials().find(|x| x.name == name).map(|x| x.id);
    found.unwrap_or_else(|| m.add_material(name, class, rgb, CutPattern::Solid))
}

/// The system's envelope types (steel frame façade of 197 mm, floor on
/// its chassis, roof) added to `m`, and the fill of a box with them and
/// the template's false ceiling.
pub fn system_fill(m: &mut Model, lib: &archisim_library::Library) -> Res<BoxFill> {
    archisim_library::frame_material(m, FrameMaterial::Steel);
    let m = &mut *m;
    let ceramic = material(m, "Placa cerámica", "ceramic", [196, 120, 92]);
    let membrane = material(m, "Membrana transpirable", "membrane", [90, 110, 140]);
    let board = material(m, "Placa con aislamiento", "insulation", [230, 220, 170]);
    let wool = material(m, "Lana mineral", "insulation", [235, 225, 150]);
    let plaster = material(m, "Placa de yeso laminado", "plaster", [240, 240, 236]);
    let wood = material(m, "Tarima de madera", "wood", [190, 150, 105]);
    let deck = material(m, "Forjado colaborante H90+30", "concrete", [150, 155, 160]);
    let decking = material(m, "Tablero de cubierta", "wood", [200, 170, 120]);
    let sheet = material(m, "Lámina impermeable", "membrane", [60, 60, 60]);
    let layer = |material, thickness, function| Layer {
        material,
        thickness,
        function,
    };
    let facade = m.add_wall_type(
        "Fachada de caja 197 (steel frame)",
        WallFunction::Exterior,
        vec![
            layer(ceramic, 0.010, LayerFunction::Finish),
            layer(membrane, 0.001, LayerFunction::Membrane),
            layer(board, 0.051, LayerFunction::Insulation),
            layer(wool, 0.100, LayerFunction::Structure),
            layer(plaster, 0.025, LayerFunction::Finish),
        ],
    )?;
    m.set_type_framing(
        facade.0,
        Some(Framing::for_depth(FrameMaterial::Steel, 0.10)),
    )?;
    let floor = m.add_slab_type(
        "Suelo de caja 270 (chasis)",
        SlabFunction::Floor,
        vec![
            layer(wood, 0.020, LayerFunction::Finish),
            layer(board, 0.059, LayerFunction::Substrate),
            layer(deck, 0.120, LayerFunction::Structure),
            layer(wool, 0.071, LayerFunction::Insulation),
        ],
    )?;
    // From the ceiling's plane (2,93) to 20 cm under the crown: the
    // ceiling frame and its joists, the board with its insulation and the
    // falls, under the parapet.
    let falls = material(m, "Mortero de pendiente", "mortar", [205, 200, 190]);
    let roof = m.add_slab_type(
        "Cubierta de caja 180",
        SlabFunction::Roof,
        vec![
            layer(sheet, 0.005, LayerFunction::Membrane),
            layer(falls, 0.030, LayerFunction::Substrate),
            layer(board, 0.059, LayerFunction::Insulation),
            layer(decking, 0.006, LayerFunction::Substrate),
            // The ceiling frame and its joists.
            layer(wool, 0.080, LayerFunction::Structure),
        ],
    )?;
    let fill = BoxFill {
        wall_type: Some(facade),
        floor_type: Some(floor),
        roof_type: Some(roof),
        ceiling_type: Some(lib.ceiling),
        ceiling_height: 2.93,
        parapet: archisim_core::BOX_PARAPET,
    };
    Ok(fill)
}

pub fn build() -> Res<Model> {
    let mut m = Model::new("Vivienda de cajas");
    let pb = m.add_level("Planta baja", 0.0);
    let lib = archisim_library::add_library(&mut m)?;
    let fill = system_fill(&mut m, &lib)?;
    let kinds = [
        "Caja 6,20 · dormitorio doble",
        "Caja 6,20 · baño + cocina",
        "Caja 6,20 · estar",
        "Caja 6,20 · estar",
        "Caja 6,20 · dormitorio",
        "Caja 6,20 · dormitorio",
        "Caja 6,20 · baño",
        "Caja 6,20 · dormitorio principal",
    ];
    // Six side by side, their length to the north (side A on the east,
    // B on the north end), and two more on the north ends of the last two.
    let step = BOX_WIDTH + JOINT;
    let quarter = std::f64::consts::FRAC_PI_2;
    let mut ids = Vec::new();
    for i in 0..6 {
        let origin = Point2::new(step * i as f64 + BOX_WIDTH, 0.0);
        ids.push(m.add_box(NewBox::usual(pb, origin, quarter))?);
    }
    for i in 4..6 {
        let origin = Point2::new(step * i as f64 + BOX_WIDTH, 6.20 + JOINT);
        ids.push(m.add_box(NewBox::usual(pb, origin, quarter))?);
    }
    for (id, kind) in ids.iter().zip(kinds) {
        m.edit_box(*id, |b| b.kind = kind.into())?;
    }
    // A first floor over the first two: their floors on those roofs, a
    // joint (2 cm) between the two chassis.
    let roof_top = BOX_BOTTOM + BOX_HEIGHT;
    let p1 = m.add_level("Planta primera", roof_top + JOINT - BOX_BOTTOM);
    let upper = [
        m.add_box(NewBox::usual(p1, Point2::new(BOX_WIDTH, 0.0), quarter))?,
        m.add_box(NewBox::usual(
            p1,
            Point2::new(step + BOX_WIDTH, 0.0),
            quarter,
        ))?,
    ];
    for (id, kind) in upper
        .iter()
        .zip(["Caja 6,20 · dormitorio", "Caja 6,20 · estudio"])
    {
        m.edit_box(*id, |b| {
            b.kind = kind.into();
            // Hung from six points: it is lifted higher, on a longer reach.
            b.chassis.lift = archisim_core::LiftPoints::Six;
        })?;
    }
    ids.extend(upper);
    for id in &ids {
        m.set_box_sides_from_neighbours(*id)?;
    }
    // The living room goes through the joint of the third and fourth
    // boxes: open on both sides.
    let (third, fourth) = (ids[2], ids[3]);
    m.edit_box(third, |b| b.sides[0] = SideState::Open)?;
    m.edit_box(fourth, |b| b.sides[2] = SideState::Open)?;
    for id in &ids {
        m.fill_box(*id, fill)?;
    }
    // The entrance in the south end of the third box, a window in the
    // first one.
    let wall_on = |m: &Model, b: archisim_core::BoxId, side: usize| {
        let s = m.modular_box(b).unwrap().side(side);
        let mid = (s.0 + s.1) * 0.5;
        m.box_members(b).walls.into_iter().find(|w| {
            let w = m.wall(*w).unwrap();
            ((w.start + w.end) * 0.5).distance(mid) < 0.05
        })
    };
    if let Some(w) = wall_on(&m, third, 3) {
        m.add_opening(NewOpening {
            opening_type: lib.entrance_door,
            wall: w,
            along: 1.8,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })?;
    }
    if let Some(w) = wall_on(&m, ids[0], 3) {
        m.add_opening(NewOpening {
            opening_type: lib.window_two_leaves,
            wall: w,
            along: 1.8,
            sill: 0.9,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })?;
    }
    Ok(m)
}

/// The boxes, what each holds and their notes, as text.
pub fn list(m: &Model) -> String {
    let mut s = String::new();
    for b in m.boxes() {
        let held = m.box_members(b.id);
        let sides: Vec<String> = (0..4)
            .map(|i| format!("{} {}", SIDE_NAMES[i], b.sides[i].code()))
            .collect();
        let _ = writeln!(
            s,
            "{} · {} · {:.2} × {:.2} × {:.2} m · {} · {} muros, {} huecos, {} forjados, {} techos",
            b.mark,
            b.kind,
            b.length,
            b.width,
            b.height,
            sides.join(", "),
            held.walls.len(),
            held.openings.len(),
            held.slabs.len(),
            held.ceilings.len()
        );
        for issue in m.box_issues(b.id) {
            let _ = writeln!(
                s,
                "    {} {}",
                if issue.is_error() { "✗" } else { "·" },
                issue.text(m)
            );
        }
    }
    s
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    archisim_file::save(&m, dir.join(format!("cajas.{}", archisim_file::EXTENSION)))?;
    std::fs::write(dir.join("cajas.ifc"), crate::ifc_text(&m, "cajas", None))?;
    let options = archisim_plan::PlanOptions {
        scale: 100.0,
        ..Default::default()
    };
    let level = m.levels()[0].id;
    std::fs::write(
        dir.join("cajas-planta.svg"),
        archisim_plan::render_plan(&m, level, &options),
    )?;
    for (n, l) in m.levels().iter().enumerate().skip(1) {
        std::fs::write(
            dir.join(format!("cajas-planta-{}.svg", n + 1)),
            archisim_plan::render_plan(&m, l.id, &options),
        )?;
    }
    let text = format!("{}\n{}", list(&m), archisim_struct::boxes::boxes_report(&m));
    std::fs::write(dir.join("cajas.txt"), &text)?;
    print!("{text}");
    println!("  {}", dir.join("cajas.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_house_of_boxes() {
        let m = build().unwrap();
        assert_eq!(m.boxes().count(), 10);
        // The upper ones over the first two, on their grid; the lower ones
        // under them without a parapet.
        let first = m.boxes().next().unwrap().id;
        assert_eq!(m.boxes_above(first).len(), 1);
        let walls: usize = m.boxes().map(|b| m.box_members(b.id).walls.len()).sum();
        // Every wall belongs to one box.
        assert_eq!(walls, m.walls().count());
        // Notes only: wide load, and the living room open on both sides.
        for b in m.boxes() {
            for i in m.box_issues(b.id) {
                assert!(!i.is_error(), "{}: {}", b.mark, i.text(&m));
            }
        }
        let text = list(&m);
        assert!(text.contains("MOD-08"), "{text}");
    }
}
