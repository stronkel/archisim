//! `archisim catalogo`: the system's catalogue (M8, `docs/MODULAR.md`).
//! Three boxes drawn in a workshop project become three box types —
//! «Caja 6,20 · dormitorio doble» (with a window the house may change),
//! «… · baño + cocina» and «… · estar abierta» —, each approved at its
//! first revision and written to `Cajas tipo/`. A house then places four
//! copies of them side by side, each one snapped against the last, the
//! last turned round and with the other window. Writes the box types, the
//! house, its IFC and its plan.

use std::path::Path;

use archisim_core::{
    BoxId, CatalogueEntry, CatalogueOption, Hand, Model, ModuleSourceId, NewBox,
    NewModulePlacement, NewOpening, Point2, SideState, WallSide, BOX_JOINT, BOX_WIDTH,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Where the box types go, inside the output folder (as in the library).
pub const BOX_TYPES: &str = "Cajas tipo";

struct Kind {
    code: &'static str,
    name: &'static str,
    /// A (bottom, along the length), B, C (top) and D.
    sides: [SideState; 4],
    /// A window in side A, or a door.
    window: bool,
    door: bool,
}

const KINDS: [Kind; 3] = [
    Kind {
        code: "CB-620-DD",
        name: "Caja 6,20 · dormitorio doble",
        sides: [
            SideState::Free,
            SideState::Free,
            SideState::Wall,
            SideState::Free,
        ],
        window: true,
        door: false,
    },
    Kind {
        code: "CB-620-BC",
        name: "Caja 6,20 · baño + cocina",
        sides: [
            SideState::Open,
            SideState::Free,
            SideState::Wall,
            SideState::Free,
        ],
        window: false,
        door: false,
    },
    Kind {
        code: "CB-620-EA",
        name: "Caja 6,20 · estar abierta",
        sides: [
            SideState::Open,
            SideState::Free,
            SideState::Open,
            SideState::Free,
        ],
        window: false,
        door: true,
    },
];

/// The wall of a box on one of its sides.
fn wall_on(m: &Model, b: BoxId, side: usize) -> Option<archisim_core::WallId> {
    let s = m.modular_box(b)?.side(side);
    let mid = (s.0 + s.1) * 0.5;
    m.box_members(b).walls.into_iter().find(|w| {
        m.wall(*w)
            .is_some_and(|w| ((w.start + w.end) * 0.5).distance(mid) < 0.05)
    })
}

/// The three box types, from a workshop project where they are drawn
/// anywhere and turned anyhow (as boxes drawn over an imported house).
pub fn box_types() -> Res<Vec<Model>> {
    let mut m = Model::new("Taller de cajas");
    let pb = m.add_level("Planta baja", 0.0);
    let lib = archisim_library::add_library(&mut m)?;
    let fill = crate::cajas::system_fill(&mut m, &lib)?;
    let window = m
        .opening_type(lib.window_two_leaves)
        .map(|t| t.name.clone())
        .ok_or("sin ventana")?;
    let others: Vec<String> = [lib.window_shutter, lib.window_one_leaf]
        .iter()
        .filter_map(|t| m.opening_type(*t).map(|t| t.name.clone()))
        .collect();
    let mut out = Vec::new();
    for (i, k) in KINDS.iter().enumerate() {
        let at = Point2::new(30.0 * i as f64 + 10.0, 5.0 + 3.0 * i as f64);
        let mut spec = NewBox::usual(pb, at, 0.3 * i as f64);
        spec.sides = k.sides;
        spec.kind = k.name.into();
        let b = m.add_box(spec)?;
        m.fill_box(b, fill)?;
        if k.window {
            let w = wall_on(&m, b, 0).ok_or("sin muro en A")?;
            m.add_opening(NewOpening {
                opening_type: lib.window_two_leaves,
                wall: w,
                along: 3.1,
                sill: 0.9,
                opens_to: WallSide::Interior,
                hand: Hand::Left,
                setback: None,
            })?;
        }
        if k.door {
            let w = wall_on(&m, b, 3).ok_or("sin muro en D")?;
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
        let mut entry = CatalogueEntry::new(k.code, k.name);
        if k.window {
            entry.options.push(CatalogueOption {
                name: "Ventana del dormitorio".into(),
                slot: window.clone(),
                choices: others.clone(),
            });
        }
        let mut t = m.box_type_model(b, entry)?;
        t.project_name = k.name.into();
        t.approve_catalogue("09/11/2026")?;
        out.push(t);
    }
    Ok(out)
}

/// The house: four copies placed one against the other.
pub fn house(types: &[(String, Model)]) -> Res<Model> {
    let mut h = Model::new("Vivienda de catálogo");
    let pb = h.add_level("Planta baja", 0.0);
    archisim_library::add_library(&mut h)?;
    let mut sources: Vec<ModuleSourceId> = Vec::new();
    for (path, t) in types {
        let text = archisim_file::to_string(t);
        let (text, stamp, model) = archisim_file::read_module(&text)?;
        let name = t.catalogue().map_or("Caja".into(), |c| c.code.clone());
        sources.push(h.add_module_source(name, path.clone(), text.into(), stamp, model)?);
    }
    let half_turn = std::f64::consts::PI;
    // Dormitorio, baño y cocina, estar, and the dormitorio again turned
    // round: each guessed a little off and snapped against the last.
    let step = BOX_WIDTH + BOX_JOINT;
    let plan = [
        (sources[0], Point2::new(0.0, 0.0), 0.0),
        (sources[1], Point2::new(0.3, step + 0.4), 0.0),
        (sources[2], Point2::new(-0.2, 2.0 * step - 0.3), 0.0),
        (
            sources[0],
            Point2::new(6.2 + 0.3, 3.0 * step + BOX_WIDTH + 0.2),
            half_turn,
        ),
    ];
    let mut last = None;
    for (i, (source, guess, rotation)) in plan.into_iter().enumerate() {
        let at = if i == 0 {
            guess
        } else {
            h.module_box_snap(source, pb, guess, rotation, false, 1.5)
                .ok_or("no se pega a la caja de al lado")?
        };
        last = Some(h.add_module(NewModulePlacement {
            source,
            level: pb,
            position: at,
            rotation,
            mirror: false,
        })?);
    }
    // The last one takes the other window.
    let choice = types[0]
        .1
        .catalogue()
        .and_then(|c| c.options.first())
        .map(|o| (o.name.clone(), o.choices[0].clone()))
        .ok_or("sin opción")?;
    h.edit_module(last.ok_or("sin copias")?, |p| p.choices = vec![choice])?;
    Ok(h)
}

pub fn write(dir: &Path) -> Res {
    let types = box_types()?;
    let folder = dir.join(BOX_TYPES);
    std::fs::create_dir_all(&folder)?;
    let mut written = Vec::new();
    for t in types {
        let code = t.catalogue().map(|c| c.code.clone()).unwrap_or_default();
        let path = folder.join(format!("{code}.{}", archisim_file::EXTENSION));
        archisim_file::save(&t, &path)?;
        println!(
            "  {} · {}",
            t.catalogue().map(|c| c.label()).unwrap_or_default(),
            path.display()
        );
        written.push((
            format!("{BOX_TYPES}/{code}.{}", archisim_file::EXTENSION),
            t,
        ));
    }
    let h = house(&written)?;
    archisim_file::save(
        &h,
        dir.join(format!("catalogo.{}", archisim_file::EXTENSION)),
    )?;
    let flat = h.flattened()?;
    std::fs::write(
        dir.join("catalogo.ifc"),
        crate::ifc_text(&flat, "catalogo", None),
    )?;
    let options = archisim_plan::PlanOptions {
        scale: 100.0,
        ..Default::default()
    };
    let level = flat.levels()[0].id;
    std::fs::write(
        dir.join("catalogo-planta.svg"),
        archisim_plan::render_plan(&flat, level, &options),
    )?;
    let text = crate::cajas::list(&flat);
    std::fs::write(dir.join("catalogo.txt"), &text)?;
    print!("{text}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_house_of_catalogue_boxes() {
        let types = box_types().unwrap();
        assert_eq!(types.len(), 3);
        for t in &types {
            assert!(t.is_frozen());
            assert_eq!(t.boxes().count(), 1);
            let b = t.boxes().next().unwrap();
            assert!(b.origin.approx_eq(Point2::new(0.0, 0.0)));
        }
        // The window of the dormitorio and its option.
        assert_eq!(types[0].openings().count(), 1);
        let named: Vec<(String, Model)> = types.into_iter().map(|t| (String::new(), t)).collect();
        let h = house(&named).unwrap();
        let flat = h.flattened().unwrap();
        let marks: Vec<&str> = flat.boxes().map(|b| b.mark.as_str()).collect();
        assert_eq!(marks, ["MOD-01", "MOD-02", "MOD-03", "MOD-04"]);
        assert!(flat.boxes().all(|b| b.kind.contains("rev. 1")));
        // Every joint has its owner: only the wide load is noted.
        for b in flat.boxes() {
            for i in flat.box_issues(b.id) {
                assert!(!i.is_error(), "{}: {}", b.mark, i.text(&flat));
            }
        }
        // The two windows of the dormitorios are different types.
        let windows: std::collections::BTreeSet<_> =
            flat.openings().map(|o| o.opening_type).collect();
        assert_eq!(flat.openings().filter(|o| o.sill > 0.5).count(), 2);
        assert!(windows.len() >= 3, "{windows:?}");
    }
}
