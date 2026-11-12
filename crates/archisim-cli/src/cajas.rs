//! `archisim cajas`: a house of modular boxes in L, as the system's
//! (`docs/MODULAR.md`): six boxes of 6,20 × 3,60 m side by side and two
//! more on the ends of the last two, 2 cm apart, and two on a first floor
//! over the first two (lifted from six points). Each box is filled with
//! its envelope (steel frame façade, floor on its chassis, roof and false
//! ceiling); the joints carry one wall, except the one of the living room,
//! open on both sides. Writes the project, its IFC (each box an
//! `IfcElementAssembly` with its parts and its chassis), the plans and a
//! list of the boxes with what they hold, their notes, their weight and
//! lifting, the order they are craned in and their cut lists; and the
//! work on site under them (M10): slab, supports and block plinth.

use std::fmt::Write as _;
use std::path::Path;

use archisim_core::{
    BoxFill, BoxProgress, BoxStage, CutPattern, FrameMaterial, Framing, Hand, Layer, LayerFunction,
    Model, NewBox, NewOpening, Point2, SideState, SlabFunction, WallFunction, WallSide, BOX_BOTTOM,
    BOX_HEIGHT, BOX_WIDTH, SIDE_NAMES,
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
    archisim_library::joint_materials(m);
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
    // How the boards are laid (M18): the insulating boards in a zipper,
    // the ceramic cladding by joint lines with wood round the windows.
    archisim_library::modular_board_layouts(m);
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
    // Where they are (M9): the first two up already, the third on the
    // lorry, the fourth at the factory's station 3, the fifth approved.
    let order = archisim_struct::boxes::assembly_order(&m);
    let stages = [
        (BoxStage::Placed, ""),
        (BoxStage::Placed, ""),
        (BoxStage::Transport, ""),
        (BoxStage::Factory, "Puesto 3"),
        (BoxStage::Approved, ""),
    ];
    for (id, (stage, station)) in order.iter().zip(stages) {
        m.edit_box(*id, |b| {
            b.progress = BoxProgress {
                stage,
                station: station.into(),
                date: "10/11/2026".into(),
            }
        })?;
    }
    // The crane, 6 m off the long side of the row.
    let corners: Vec<Point2> = m.boxes().flat_map(|b| b.corners()).collect();
    let (min_x, max_x) = corners
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, z), p| (a.min(p.x), z.max(p.x)));
    let min_y = corners.iter().map(|p| p.y).fold(f64::MAX, f64::min);
    let mut site = m.modular_site();
    site.crane = Some(Point2::new((min_x + max_x) / 2.0, min_y - 6.0));
    m.set_modular_site(site)?;
    // The work on site before the boxes come (M10): slab, supports and
    // plinth, on their level under the boxes.
    m.build_box_base(&archisim_core::BaseOptions::default())?;
    // A porch on the first free long side of a box on the ground (M17).
    let spot = m.ground_boxes().into_iter().find_map(|b| {
        [0usize, 2]
            .into_iter()
            .find(|s| {
                b.sides[*s] == archisim_core::SideState::Free
                    && m.box_contacts(b.id).iter().all(|k| k.side != *s)
            })
            .map(|s| (b.id, s))
    });
    if let Some((b, side)) = spot {
        m.build_porch(
            b,
            &archisim_core::PorchOptions {
                side,
                depth: 2.4,
                ..Default::default()
            },
        )?;
    }
    // One façade composed by hand (M20): the longest wall clad in boards,
    // in panels of 1,20 m, the last piece by the corner fitted on site.
    let face = m
        .walls()
        .filter_map(|w| {
            let layer = archisim_geom::boards::wall_boards(&m, w.id)
                .first()
                .map(|p| p.layer)?;
            Some((w.id, layer, w.length(), w.base_offset, w.height))
        })
        .max_by(|a, b| a.2.total_cmp(&b.2));
    if let Some((wall, layer, len, base, height)) = face {
        let mut joints = Vec::new();
        let mut x = 1.2;
        while x < len - 0.3 {
            joints.push((Point2::new(x, base), Point2::new(x, base + height)));
            x += 1.2;
        }
        m.set_board_face(
            wall,
            layer,
            Some(archisim_core::BoardFace {
                wall,
                layer,
                joints,
                marks: vec![archisim_core::PieceMark {
                    at: Point2::new(len - 0.1, base + height / 2.0),
                    material: None,
                    on_site: true,
                }],
                joint: None,
            }),
        )?;
    }
    // And one floor in plan: its top boards across, every 1,22 m.
    let floor = m.slabs().find_map(|s| {
        let layer = archisim_geom::boards::slab_boards(&m, s.id).first()?.layer;
        Some((s.id, layer, s.outline.clone()))
    });
    if let Some((slab, layer, outline)) = floor {
        let (x0, x1) = outline
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
        let (y0, y1) = outline
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.y), b.max(p.y)));
        let mut joints = Vec::new();
        let mut x = x0 + 1.22;
        while x < x1 - 0.3 {
            joints.push((Point2::new(x, y0 - 0.1), Point2::new(x, y1 + 0.1)));
            x += 1.22;
        }
        let host = archisim_core::PlanHost::Slab(slab);
        m.set_board_plan(
            host,
            layer,
            Some(archisim_core::BoardPlan {
                host,
                layer,
                joints,
                marks: Vec::new(),
                joint: None,
            }),
        )?;
    }
    Ok(m)
}

/// The workshop drawings of a project's boxes (M9): `<stem>-taller.pdf`,
/// A3, and each page as `<stem>-taller-M-nn.svg`. The number of pages.
pub fn workshop(m: &Model, dir: &Path, stem: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let pages = archisim_plan::box_sheets::pages(m);
    if pages.is_empty() {
        return Ok(0);
    }
    let codes = archisim_plan::box_sheets::codes(pages.len());
    for (svg, code) in pages.iter().zip(&codes) {
        std::fs::write(dir.join(format!("{stem}-taller-{code}.svg")), svg)?;
    }
    let pdf_pages: Vec<archisim_export::pdf::Page> = pages
        .iter()
        .zip(&codes)
        .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
        .collect();
    std::fs::write(
        dir.join(format!("{stem}-taller.pdf")),
        archisim_export::pdf::pdf(&pdf_pages, "Planos de taller de las cajas")?,
    )?;
    Ok(pages.len())
}

/// The drawings of the faces composed by hand (M20): one SVG per sheet and
/// the PDF. Returns how many sheets.
pub fn board_drawings(
    m: &Model,
    dir: &Path,
    stem: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let pages = archisim_plan::board_sheets::pages(m);
    if pages.is_empty() {
        return Ok(0);
    }
    let codes = archisim_plan::board_sheets::codes(pages.len());
    for (svg, code) in pages.iter().zip(&codes) {
        std::fs::write(dir.join(format!("{stem}-despiece-{code}.svg")), svg)?;
    }
    let pdf_pages: Vec<archisim_export::pdf::Page> = pages
        .iter()
        .zip(&codes)
        .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
        .collect();
    std::fs::write(
        dir.join(format!("{stem}-despiece.pdf")),
        archisim_export::pdf::pdf(&pdf_pages, "Despiece a mano de las fachadas")?,
    )?;
    Ok(pages.len())
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
    // The same in IFC4, for the programs that do not read 4.3.
    std::fs::write(
        dir.join("cajas-ifc4.ifc"),
        crate::ifc_text_in(&m, "cajas-ifc4", None, archisim_ifc::Schema::Ifc4),
    )?;
    let options = archisim_plan::PlanOptions {
        scale: 100.0,
        ..Default::default()
    };
    // The boxes' levels, and the base's under them.
    let mut n = 0;
    for l in m.levels() {
        let name = if l.name == archisim_core::BASE_LEVEL {
            "cajas-cimentacion.svg".to_string()
        } else {
            n += 1;
            if n == 1 {
                "cajas-planta.svg".to_string()
            } else {
                format!("cajas-planta-{n}.svg")
            }
        };
        // The base's plan, cut through its piers.
        let options = archisim_plan::PlanOptions {
            cut_plane: if l.name == archisim_core::BASE_LEVEL {
                archisim_core::BASE_CUT
            } else {
                options.cut_plane
            },
            ..options.clone()
        };
        std::fs::write(
            dir.join(name),
            archisim_plan::render_plan(&m, l.id, &options),
        )?;
    }
    let text = format!("{}\n{}", list(&m), archisim_struct::boxes::boxes_report(&m));
    std::fs::write(dir.join("cajas.txt"), &text)?;
    let sheets = workshop(&m, dir, "cajas")?;
    let faces = board_drawings(&m, dir, "cajas")?;
    println!(
        "  Despiece a mano: {faces} hojas A3 en {}",
        dir.join("cajas-despiece.pdf").display()
    );
    print!("{text}");
    println!(
        "  Planos de taller: {} hojas A3 en {}",
        sheets,
        dir.join("cajas-taller.pdf").display()
    );
    // The boards piece by piece and the shopping list (M14).
    std::fs::write(
        dir.join("cajas-tableros.csv"),
        archisim_geom::boards::boards_csv(&m),
    )?;
    std::fs::write(
        dir.join("cajas-compra.csv"),
        archisim_geom::boards::shopping_csv(&m),
    )?;
    println!("  {}", dir.join("cajas-compra.csv").display());
    // The factory (M16).
    print!("{}", crate::fabrica::write(&m, dir, "cajas")?);
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
        // Every wall belongs to one box, but the plinth's, on the base's
        // level.
        let base = m
            .levels()
            .into_iter()
            .find(|l| l.name == archisim_core::BASE_LEVEL)
            .unwrap()
            .id;
        let plinth = m.walls().filter(|w| w.level == base).count();
        assert_eq!(plinth, 6, "an L of six sides");
        assert_eq!(walls + plinth, m.walls().count());
        // Under the eight boxes on the ground, their supports.
        let check = archisim_struct::boxes::base_check(&m).unwrap();
        assert_eq!(
            m.columns().filter(|c| c.level == base).count(),
            check.supports.len()
        );
        assert!(check.ground_passes(), "{}", check.pressure);
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
