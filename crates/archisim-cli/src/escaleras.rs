//! `archisim escaleras`: every kind of stair ArchiSim makes, side by
//! side, each up to the floor above: straight, in L and in U with a
//! landing or with winders, spiral round a newel and with a helicoidal
//! slab, on a slab, on stringers, on a central stringer and built into a
//! wall, with closed and open risers, with split landings and a
//! spiral's landing at its head; to see them in plan, in 3D and in
//! the IFC.

use std::path::Path;

use archisim_core::{
    preset_stair, MaterialId, Model, NewStair, NewWall, Point2, PresetSpec, ReferenceLine,
    StairDesign, StairPreset, StairStructure, StairUse, Vec2, WallTop,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// One stair of the sample: its name, shape and build.
struct Sample {
    name: &'static str,
    preset: StairPreset,
    winders: bool,
    structure: StairStructure,
    closed: bool,
    /// Wooden treads (or a stone finish on a slab).
    treads: bool,
    /// Inside a dwelling (CTE "uso restringido").
    dwelling: bool,
    /// Landings split at 45°; a spiral's landing at its head.
    split: bool,
    arrival: bool,
}

const SAMPLES: [Sample; 12] = [
    Sample {
        name: "Recta de losa",
        preset: StairPreset::Straight,
        winders: false,
        structure: StairStructure::Slab,
        closed: true,
        treads: false,
        dwelling: false,
        split: false,
        arrival: false,
    },
    Sample {
        name: "En L con meseta, peldañeado de piedra",
        preset: StairPreset::LShape,
        winders: false,
        structure: StairStructure::Slab,
        closed: true,
        treads: true,
        dwelling: false,
        split: false,
        arrival: false,
    },
    Sample {
        name: "En L compensada",
        preset: StairPreset::LShape,
        winders: true,
        structure: StairStructure::Slab,
        closed: true,
        treads: false,
        dwelling: false,
        split: false,
        arrival: false,
    },
    Sample {
        name: "En U con meseta",
        preset: StairPreset::UShape,
        winders: false,
        structure: StairStructure::Slab,
        closed: true,
        treads: false,
        dwelling: false,
        split: false,
        arrival: false,
    },
    Sample {
        name: "En U compensada, zancas de acero (vivienda)",
        preset: StairPreset::UShape,
        winders: true,
        structure: StairStructure::Stringers,
        closed: false,
        treads: true,
        dwelling: true,
        split: false,
        arrival: false,
    },
    Sample {
        name: "De caracol con mástil y meseta de llegada",
        preset: StairPreset::Spiral,
        winders: false,
        structure: StairStructure::Newel,
        closed: false,
        treads: true,
        dwelling: true,
        split: false,
        arrival: true,
    },
    Sample {
        name: "Helicoidal de losa",
        preset: StairPreset::Spiral,
        winders: false,
        structure: StairStructure::Slab,
        closed: true,
        treads: false,
        dwelling: true,
        split: false,
        arrival: false,
    },
    Sample {
        name: "Recta de zanca central",
        preset: StairPreset::Straight,
        winders: false,
        structure: StairStructure::Spine,
        closed: false,
        treads: true,
        dwelling: false,
        split: false,
        arrival: false,
    },
    Sample {
        name: "En L de peldaños volados",
        preset: StairPreset::LShape,
        winders: false,
        structure: StairStructure::Cantilever,
        closed: false,
        treads: true,
        dwelling: false,
        split: false,
        arrival: false,
    },
    Sample {
        name: "Recta de zancas con tabica",
        preset: StairPreset::Straight,
        winders: false,
        structure: StairStructure::Stringers,
        closed: true,
        treads: true,
        dwelling: false,
        split: false,
        arrival: false,
    },
    Sample {
        name: "En L con meseta partida (vivienda)",
        preset: StairPreset::LShape,
        winders: false,
        structure: StairStructure::Slab,
        closed: true,
        treads: true,
        dwelling: true,
        split: true,
        arrival: false,
    },
    Sample {
        name: "En U con mesetas partidas, zancas (vivienda)",
        preset: StairPreset::UShape,
        winders: false,
        structure: StairStructure::Stringers,
        closed: true,
        treads: true,
        dwelling: true,
        split: true,
        arrival: false,
    },
];

fn material(m: &Model, category: &str) -> Option<MaterialId> {
    m.materials().find(|x| x.category == category).map(|x| x.id)
}

pub fn build() -> Res<Model> {
    let mut m = Model::new("Escaleras de ArchiSim");
    let pb = m.add_level("Planta baja", 0.0);
    let p1 = m.add_level("Planta primera", 2.97);
    let lib = archisim_library::add_library(&mut m)?;
    let concrete = material(&m, "concrete").ok_or("sin hormigón")?;
    let steel = material(&m, "steel").unwrap_or(concrete);
    let wood = material(&m, "wood").unwrap_or(concrete);
    let stone = material(&m, "stone").unwrap_or(wood);
    for (i, s) in SAMPLES.iter().enumerate() {
        let foot = Point2::new((i % 5) as f64 * 7.0, (i / 5) as f64 * -9.0);
        let spiral = s.preset == StairPreset::Spiral;
        let usage = if s.dwelling {
            StairUse::Restricted
        } else {
            StairUse::General
        };
        let (width, going) = match (spiral, s.dwelling) {
            (true, _) => (0.8, 0.24),
            (false, true) => (0.9, 0.25),
            _ => (1.0, 0.28),
        };
        // The helicoidal slab round an eye; the newel, thin.
        let core = if s.structure == StairStructure::Slab {
            0.5
        } else {
            0.12
        };
        let spec = PresetSpec {
            preset: s.preset,
            winders: s.winders,
            split: s.split,
            left: true,
            width,
            going,
            riser: 0.175,
            height: 2.97,
            gap: 0.2,
            core,
            edge: archisim_core::walking_edge(usage, width),
        };
        let start = if spiral {
            foot + Vec2::new(2.0, 2.0)
        } else {
            foot
        };
        let (path, form, sweep) = preset_stair(&spec, start, Vec2::new(0.0, 1.0)).ok_or(s.name)?;
        let slab = s.structure == StairStructure::Slab;
        let id = m.add_stair(NewStair {
            level: pb,
            path,
            width,
            going,
            base_offset: 0.0,
            top: WallTop::Level {
                level: p1,
                offset: 0.0,
            },
            height: 0.0,
            waist: 0.15,
            material: if slab { concrete } else { steel },
            usage,
            design: StairDesign {
                form,
                sweep,
                core,
                structure: s.structure,
                closed_risers: s.closed,
                tread: if slab { 0.03 } else { 0.05 },
                tread_material: s.treads.then_some(if slab { stone } else { wood }),
                split: s.split,
                arrival: if s.arrival {
                    std::f64::consts::FRAC_PI_2
                } else {
                    0.0
                },
                ..Default::default()
            },
        })?;
        if s.structure == StairStructure::Cantilever {
            // Built into the wall on its outer side (its right, walking
            // up): the railing on the other.
            m.edit_stair(id, |x| x.railings = archisim_core::Railings::Left)?;
            let l = m.stair(id).ok_or("escalera")?.layout();
            let side: Vec<Point2> = l.side(false).iter().map(|p| p.p).collect();
            let mut corners = vec![side[0]];
            for w in side.windows(3) {
                let (a, b) = ((w[1] - w[0]).normalized(), (w[2] - w[1]).normalized());
                if let (Some(a), Some(b)) = (a, b) {
                    if a.cross(b).abs() > 0.1 {
                        corners.push(w[1]);
                    }
                }
            }
            corners.push(side[side.len() - 1]);
            for w in corners.windows(2) {
                m.add_wall(NewWall {
                    wall_type: lib.bearing_wall,
                    level: pb,
                    start: w[0],
                    end: w[1],
                    reference: ReferenceLine::ExteriorFace,
                    height: 2.97,
                    base_offset: 0.0,
                    bulge: 0.0,
                    top: WallTop::Level {
                        level: p1,
                        offset: 0.0,
                    },
                })?;
            }
        }
    }
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    let m = build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("escaleras.{}", archisim_file::EXTENSION));
    archisim_file::save(&m, &project)?;
    std::fs::write(
        dir.join("escaleras.ifc"),
        crate::ifc_text(&m, "escaleras", None),
    )?;
    let axo = archisim_core::Axo::default();
    let drawing = archisim_plan::axo::axo_drawing(&m, &axo, Default::default());
    let crop = archisim_plan::axo::axo_extent(&m, &axo, 50.0);
    std::fs::write(
        dir.join("escaleras-3d.svg"),
        archisim_plan::axo::render_axo(&drawing, crop, 50.0),
    )?;
    let options = archisim_plan::PlanOptions {
        scale: 50.0,
        ..Default::default()
    };
    for (i, level) in m.levels().iter().enumerate() {
        std::fs::write(
            dir.join(format!("escaleras-planta-{i}.svg")),
            archisim_plan::render_plan(&m, level.id, &options),
        )?;
    }
    println!(
        "Escaleras: {} tipos, cada una hasta la planta primera.",
        m.stairs().count()
    );
    for (s, st) in SAMPLES.iter().zip(m.stairs()) {
        let l = st.layout();
        let issues = st.issues();
        println!(
            "  {}: {} tabicas de {:.1} cm{}",
            s.name,
            l.risers(),
            l.riser * 100.0,
            if issues.is_empty() {
                ", cumple el CTE".to_string()
            } else {
                format!(", {} avisos del CTE", issues.len())
            }
        );
    }
    println!("  Proyecto: {}", project.display());
    println!("  Axonometría: {}", dir.join("escaleras-3d.svg").display());
    println!(
        "  Plantas: {}",
        dir.join("escaleras-planta-0.svg").display()
    );
    println!("  IFC: {}", dir.join("escaleras.ifc").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Every stair of the sample is built, up to the floor above, and
    /// comes back the same from its file.
    #[test]
    fn every_stair_is_built() {
        let m = super::build().unwrap();
        assert_eq!(m.stairs().count(), super::SAMPLES.len());
        for s in m.stairs() {
            assert_eq!(s.layout().risers(), 17, "{:?}", s.design);
            assert!(!archisim_geom::solid::stair_solids(&m, s.id).is_empty());
        }
        let text = archisim_file::to_string(&m);
        assert_eq!(archisim_file::from_str(&text).unwrap(), m);
    }
}
