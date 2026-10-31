//! `archisim fachadas`: the façades laid piece by piece (E9), one small
//! two-storey block for each façade type of the template, side by side,
//! with windows and a door (lintels of bricks on end), bonded corners and
//! the bands over the edges of the slabs; to see them in 3D, in the
//! elevations and in the IFC, and their pieces counted.

use std::path::Path;

use archisim_core::{
    Hand, Model, NewOpening, NewSlab, NewWall, Point2, ReferenceLine, WallSide, WallTop,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Width and depth of each block, and the height of a storey.
const W: f64 = 4.2;
const D: f64 = 3.0;
const H: f64 = 3.0;

pub fn build() -> Res<Model> {
    let mut m = Model::new("Fachadas de ArchiSim");
    let ground = m.add_level("Planta baja", 0.0);
    let first = m.add_level("Planta primera", H);
    let roof = m.add_level("Cubierta", 2.0 * H);
    let lib = archisim_library::add_library(&mut m)?;
    let types = archisim_library::facades::add_facade_types(&mut m)?;
    for (i, t) in types.iter().enumerate() {
        let x0 = i as f64 * (W + 1.5);
        // Clockwise, so the exterior faces are outside.
        let c = [
            Point2::new(x0, 0.0),
            Point2::new(x0, D),
            Point2::new(x0 + W, D),
            Point2::new(x0 + W, 0.0),
        ];
        let first_layer = m
            .wall_type(*t)
            .and_then(|t| t.layers.first())
            .map_or(0.1, |l| l.thickness);
        for (level, storey) in [(ground, 0), (first, 1)] {
            let mut walls = Vec::new();
            for k in 0..4 {
                walls.push(m.add_wall(NewWall {
                    wall_type: *t,
                    level,
                    start: c[k],
                    end: c[(k + 1) % 4],
                    reference: ReferenceLine::ExteriorFace,
                    height: H,
                    base_offset: 0.0,
                    bulge: 0.0,
                    top: WallTop::Unconnected,
                })?);
            }
            // The south wall (the last one, east to west): two windows
            // upstairs, a door and a window downstairs.
            let south = walls[3];
            let window = lib.window_two_leaves;
            let openings: &[(archisim_core::OpeningTypeId, f64, f64)] = if storey == 0 {
                &[(lib.entrance_door, 1.1, 0.0), (window, 3.0, 0.9)]
            } else {
                &[(window, 1.1, 0.9), (window, 3.0, 0.9)]
            };
            for (ty, along, sill) in openings {
                m.add_opening(NewOpening {
                    opening_type: *ty,
                    wall: south,
                    along: *along,
                    sill: *sill,
                    opens_to: WallSide::Interior,
                    hand: Hand::Left,
                    setback: None,
                })?;
            }
            // A window in the east wall.
            m.add_opening(NewOpening {
                opening_type: lib.window_one_leaf,
                wall: walls[2],
                along: D / 2.0,
                sill: 0.9,
                opens_to: WallSide::Interior,
                hand: Hand::Left,
                setback: None,
            })?;
        }
        // The floors reach the back of the façade's outer leaf, so their
        // edges are behind it.
        let inset = first_layer;
        let floor = vec![
            Point2::new(x0 + inset, inset),
            Point2::new(x0 + W - inset, inset),
            Point2::new(x0 + W - inset, D - inset),
            Point2::new(x0 + inset, D - inset),
        ];
        for (level, slab_type) in [(first, lib.floor_slab), (roof, lib.flat_roof)] {
            m.add_slab(NewSlab {
                slab_type,
                level,
                outline: floor.clone(),
                top_offset: 0.0,
            })?;
        }
    }
    // The south elevation of the first block, and a section through it.
    m.add_section(archisim_core::NewSection {
        name: "Sur".into(),
        kind: archisim_core::SectionKind::Elevation,
        start: Point2::new(-1.0, -3.0),
        end: Point2::new(W + 1.0, -3.0),
        depth: 5.0,
    })?;
    m.add_section(archisim_core::NewSection {
        name: "F".into(),
        kind: archisim_core::SectionKind::Section,
        start: Point2::new(2.2, -1.0),
        end: Point2::new(2.2, D + 1.0),
        depth: 3.0,
    })?;
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("fachadas.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(
        dir.join("fachadas.ifc"),
        crate::ifc_text(&m, "fachadas", None),
    )?;
    let idx = archisim_geom::WallIndex::new(&m);
    let mut whole = 0;
    let mut cut = 0;
    for w in m.walls() {
        let solids = archisim_geom::solid::wall_solids_in(&idx, w.id);
        if let Some(l) = archisim_geom::facade::facade_layout(&idx, w.id, &solids) {
            for (_, _, a, b) in archisim_geom::facade::piece_counts(&l) {
                whole += a;
                cut += b;
            }
        }
    }
    println!(
        "Fachadas: {} tipos de fachada, {whole} piezas enteras y {cut} cortadas.",
        archisim_library::facades::FACADE_TYPES.len()
    );
    println!("  Proyecto: {}", project.display());
    // The south elevation and a section at 1:50, with the joints (E17).
    let options = archisim_plan::PlanOptions {
        scale: 50.0,
        ..Default::default()
    };
    for (name, file) in [
        ("Sur", "fachadas-alzado.svg"),
        ("F", "fachadas-seccion.svg"),
    ] {
        if let Some(sec) = m.sections().find(|s| s.name == name) {
            let d = archisim_plan::section::section_drawing(&m, sec);
            std::fs::write(
                dir.join(file),
                archisim_plan::section::render_section(&m, &d, &options),
            )?;
            println!("  {name} a 1:50: {}", dir.join(file).display());
        }
    }
    println!("  IFC: {}", dir.join("fachadas.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_facade_lays_its_pieces() {
        let m = build().unwrap();
        let idx = archisim_geom::WallIndex::new(&m);
        let mut fronts = 0;
        for w in m.walls() {
            let solids = archisim_geom::solid::wall_solids_in(&idx, w.id);
            let l = archisim_geom::facade::facade_layout(&idx, w.id, &solids)
                .expect("every wall has a façade");
            assert!(!l.pieces.is_empty());
            fronts += l.pieces.iter().filter(|p| p.slab_front).count();
            // Nothing on the doors and windows.
            for o in idx.openings_in_wall(w.id) {
                let (t0, t1) = o.cut_span();
                let level = m.level(w.level).unwrap().elevation;
                let (y0, y1) = (level + o.sill, level + o.head);
                let centre = Point2::new((t0 + t1) / 2.0, (y0 + y1) / 2.0);
                for p in &l.pieces {
                    let corner_in = p.outline.iter().any(|q| {
                        q.x > t0 + 0.01 && q.x < t1 - 0.01 && q.y > y0 + 0.01 && q.y < y1 - 0.01
                    });
                    let over = archisim_core::point_in_polygon(centre, &p.outline);
                    assert!(
                        !corner_in && !over,
                        "a piece in an opening of {:?}: {p:?}",
                        w.id
                    );
                }
            }
        }
        assert!(fronts > 0, "the slab edges have their band");
    }

    /// Pictures of each façade from the south-west, close:
    /// `ARCHISIM_FACHADAS_OUT=carpeta cargo test --release -p archisim-cli facade_pictures`.
    #[test]
    fn facade_pictures() {
        let Some(dir) = std::env::var_os("ARCHISIM_FACHADAS_OUT") else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let m = build().unwrap();
        let only: Option<usize> = std::env::var("ARCHISIM_FACHADA")
            .ok()
            .and_then(|s| s.parse().ok());
        for i in 0..archisim_library::facades::FACADE_TYPES.len() {
            if only.is_some_and(|o| o != i) {
                continue;
            }
            let x0 = i as f64 * (W + 1.5);
            // ARCHISIM_FACHADA_CERCA: the corner over the door, close.
            let close = std::env::var_os("ARCHISIM_FACHADA_CERCA").is_some();
            let view = archisim_render::View {
                eye: if close {
                    [x0 + 0.6, -2.2, 3.4]
                } else {
                    [x0 - 2.5, -6.5, 3.4]
                },
                target: if close {
                    [x0 + 0.4, 0.0, 2.6]
                } else {
                    [x0 + W * 0.45, 0.0, 2.6]
                },
                fov_y: if close { 0.9 } else { 0.75 },
                perspective: true,
                half_height: 3.0,
            };
            let opt = archisim_render::Options {
                width: 900,
                height: 640,
                samples: 1,
                sky_rays: 2,
                ..Default::default()
            };
            let img = archisim_render::render(&m, &view, &opt, &|_| {});
            let bytes = archisim_export::png::encode_rgb(
                img.width as u32,
                img.height as u32,
                &img.rgb,
                96.0,
            );
            std::fs::write(dir.join(format!("fachada-{i}.png")), bytes).unwrap();
        }
    }
}
