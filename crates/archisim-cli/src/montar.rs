//! Builds a project from a plain-text description of a building
//! ("montar"): levels, walls by thickness, doors and windows, curtain walls,
//! slabs and rooms. The description is what a converter from CAD drawings
//! (double-line walls, door swings, room outlines) writes; one line per
//! element, fields separated by `|`:
//!
//! ```text
//! LEVEL|name|elevation
//! WALL|level|top level or -|thickness|kind|x1|y1|x2|y2|height if top is -
//! OPEN|wall index|door, door2, window, fixed or void|width|height|sill|along|ext or int|L or R
//! CURTAIN|level|top level|x1|y1|x2|y2
//! SLAB|level|solera, forjado or cubierta|x,y x,y …
//! HOLE|x,y x,y …            (a hole in the slab just above)
//! ROOM|level|number|name|label x,y|x,y x,y …|area in the drawing|holes (x,y … ; x,y …)
//! COLUMN|level|top level|diameter|x|y
//! LOUVRE|level|x|y|turn (radians)|width|height
//! PLANTER|level|height|thickness|centre line x,y …|earth x,y …|species;species…|plants
//! ```
//!
//! A wall's kind is its finish: `blanco` (white render outside),
//! `madera` (timber cladding outside), `int` (plaster both faces), `fence`
//! (rendered both faces) and the older `ext`, as `blanco`. The outside of
//! an exterior wall is on the left of its line, as ArchiSim's walls.
//! Slabs are `solera`, `forjado`, `volado` (white concrete with its edge
//! seen, for cantilevers), `cubierta` and `grava` (inverted flat roof with
//! gravel). Louvre panels are the «Panel de lamas orientable» object; a
//! planter is a white parapet along its centre line, the earth inside and
//! a planting of those species on it.
//!
//! Walls are drawn on their centre line and go from their level up to the
//! top level. Doors and windows are measured from the start of the wall.
//! Whatever does not fit is skipped and reported, so a drawing with odd
//! bits still gives a model.

use std::collections::BTreeMap;
use std::path::Path;

use archisim_core::{
    Arrangement, CutPattern, Hand, Layer, LayerFunction, LevelId, MaterialId, MemberTypeId,
    MemberUsage, Model, NewColumn, NewCurtainWall, NewObject, NewOpening, NewOpeningType,
    NewPlanting, NewRoom, NewSlab, NewWall, ObjectTypeId, OpeningKind, OpeningTypeId, ParamScope,
    Point2, Profile, ReferenceLine, SlabFunction, SlabTypeId, Value, WallFunction, WallSide,
    WallTop, WallTypeId,
};

type R<T> = Result<T, Box<dyn std::error::Error>>;

/// What was built and what was left out.
#[derive(Default, Debug)]
pub struct Report {
    pub walls: usize,
    pub openings: usize,
    pub curtains: usize,
    pub slabs: usize,
    pub rooms: usize,
    pub columns: usize,
    pub louvres: usize,
    pub planters: usize,
    pub plants: usize,
    pub skipped: Vec<String>,
}

fn num(s: &str) -> R<f64> {
    s.trim()
        .parse::<f64>()
        .map_err(|e| format!("número «{s}»: {e}").into())
}

fn points(s: &str) -> R<Vec<Point2>> {
    s.split_whitespace()
        .map(|p| {
            let (x, y) = p.split_once(',').ok_or("punto sin coma")?;
            Ok(Point2::new(num(x)?, num(y)?))
        })
        .collect()
}

/// A material of the palette by its name, or the project's of that name,
/// or a new one of this category and colour.
fn material(m: &mut Model, name: &str, category: &str, rgb: [u8; 3]) -> MaterialId {
    if let Some(x) = m.materials().find(|x| x.name == name) {
        return x.id;
    }
    if let Some(id) = archisim_library::palette_material(m, name) {
        return id;
    }
    m.add_material(name, category, rgb, CutPattern::Solid)
}

fn layer(material: MaterialId, thickness: f64, function: LayerFunction) -> Layer {
    Layer {
        material,
        thickness,
        function,
    }
}

/// The slab types the description names, made the first time.
fn slab_type(
    m: &mut Model,
    made: &mut BTreeMap<String, SlabTypeId>,
    kind: &str,
    lib: &archisim_library::Library,
) -> R<SlabTypeId> {
    if let Some(id) = made.get(kind) {
        return Ok(*id);
    }
    let concrete = material(m, "Hormigón visto blanco", "concrete", [236, 234, 228]);
    let id = match kind {
        "solera" => lib.ground_slab,
        "cubierta" => lib.flat_roof,
        "volado" => {
            let floor = material(m, "Microcemento", "mortar", [214, 208, 198]);
            m.add_slab_type(
                "Forjado volado de hormigón blanco 35 cm",
                SlabFunction::Floor,
                vec![
                    layer(floor, 0.02, LayerFunction::Finish),
                    layer(concrete, 0.33, LayerFunction::Structure),
                ],
            )?
        }
        "grava" => {
            let gravel = archisim_library::ground::cover_named("Grava")
                .map(|c| archisim_library::ground::cover_material(m, c))
                .unwrap_or_else(|| material(m, "Grava", "gravel", [190, 186, 176]));
            let xps = material(m, "Poliestireno extruido", "insulation", [180, 214, 236]);
            let sheet = material(m, "Lámina impermeable", "membrane", [60, 60, 64]);
            let screed = material(m, "Hormigón de pendientes", "concrete", [200, 198, 192]);
            m.add_slab_type(
                "Cubierta plana invertida con grava 50 cm",
                SlabFunction::Roof,
                vec![
                    layer(gravel, 0.05, LayerFunction::Finish),
                    layer(xps, 0.08, LayerFunction::Insulation),
                    layer(sheet, 0.01, LayerFunction::Membrane),
                    layer(screed, 0.06, LayerFunction::Substrate),
                    layer(concrete, 0.30, LayerFunction::Structure),
                ],
            )?
        }
        "tierra" => {
            let earth = material(m, "Tierra", "earth", [120, 92, 64]);
            m.add_slab_type(
                "Tierra vegetal de jardinera",
                SlabFunction::Floor,
                vec![layer(earth, 0.40, LayerFunction::Substrate)],
            )?
        }
        _ => lib.floor_slab,
    };
    made.insert(kind.to_string(), id);
    Ok(id)
}

/// Builds the project described by `text`.
pub fn build(name: &str, text: &str) -> R<(Model, Report)> {
    let mut m = Model::new(name);
    let lib = archisim_library::add_library(&mut m)?;
    let mut report = Report::default();
    let mut levels: BTreeMap<String, LevelId> = BTreeMap::new();

    // Materials for the walls: the brick of the template's partition for
    // thin walls, the one of its load-bearing wall for the rest.
    let core = |m: &Model, t: WallTypeId| {
        m.wall_type(t).and_then(|t| {
            t.layers
                .iter()
                .max_by(|a, b| a.thickness.total_cmp(&b.thickness))
                .map(|l| l.material)
        })
    };
    let thin_brick = core(&m, lib.partition).ok_or("la plantilla no tiene tabique")?;
    let thick_brick = core(&m, lib.bearing_wall).ok_or("la plantilla no tiene muro")?;
    let plaster = m
        .wall_type(lib.partition)
        .and_then(|t| {
            t.layers
                .iter()
                .find(|l| l.function == LayerFunction::Finish)
                .map(|l| l.material)
        })
        .unwrap_or(thin_brick);
    let door_type = m
        .opening_type(lib.interior_door)
        .cloned()
        .ok_or("la plantilla no tiene puerta")?;
    let window_type = m
        .opening_type(lib.window_two_leaves)
        .cloned()
        .ok_or("la plantilla no tiene ventana")?;

    let mut wall_types: BTreeMap<(i64, String), WallTypeId> = BTreeMap::new();
    let mut slab_types: BTreeMap<String, SlabTypeId> = BTreeMap::new();
    let mut column_types: BTreeMap<i64, MemberTypeId> = BTreeMap::new();
    let mut louvre_types: BTreeMap<(i64, i64), ObjectTypeId> = BTreeMap::new();
    let mut opening_types: BTreeMap<(String, i64, i64), OpeningTypeId> = BTreeMap::new();
    let mut walls: Vec<Option<archisim_core::WallId>> = Vec::new();
    let mut last_slab: Option<archisim_core::SlabId> = None;

    for (n, line) in text.lines().enumerate() {
        let f: Vec<&str> = line.split('|').collect();
        let level = |levels: &BTreeMap<String, LevelId>, s: &str| -> R<LevelId> {
            levels
                .get(s)
                .copied()
                .ok_or_else(|| format!("línea {}: no hay planta «{s}»", n + 1).into())
        };
        match f[0] {
            "LEVEL" => {
                let id = m.add_level(f[1], num(f[2])?);
                levels.insert(f[1].to_string(), id);
            }
            "WALL" => {
                let base = level(&levels, f[1])?;
                // «-» as the top: a free-standing wall of the height given
                // last (a fence, a parapet).
                let top = if f[2] == "-" {
                    WallTop::Unconnected
                } else {
                    WallTop::Level {
                        level: level(&levels, f[2])?,
                        offset: 0.0,
                    }
                };
                let height = f.get(9).map(|h| num(h)).transpose()?.unwrap_or(3.0);
                let t = num(f[3])?;
                let kind = match f[4] {
                    "ext" => "blanco",
                    k => k,
                };
                let cm = (t * 100.0).round() as i64;
                let wall_type = match wall_types.get(&(cm, kind.to_string())) {
                    Some(id) => *id,
                    None => {
                        let (function, label) = match kind {
                            "blanco" => (WallFunction::Exterior, "Fachada enfoscada en blanco"),
                            "madera" => (WallFunction::Exterior, "Fachada revestida de madera"),
                            "fence" => (WallFunction::Exterior, "Cerramiento enfoscado"),
                            "peto" => (WallFunction::Exterior, "Peto de jardinera"),
                            _ => (WallFunction::Interior, "Tabique"),
                        };
                        let render = material(&mut m, "Cal blanca", "plaster", [244, 242, 236]);
                        let wood = material(&mut m, "Teca", "wood", [150, 100, 60]);
                        let concrete =
                            material(&mut m, "Hormigón visto blanco", "concrete", [236, 234, 228]);
                        // Brick with 1.5 cm of finish on each face (2 cm of
                        // timber outside) when it is thick enough for them.
                        let brick = if t < 0.2 { thin_brick } else { thick_brick };
                        let layers = match kind {
                            "peto" => vec![layer(concrete, t, LayerFunction::Structure)],
                            _ if t < 0.09 => vec![layer(brick, t, LayerFunction::Structure)],
                            "blanco" => vec![
                                layer(render, 0.015, LayerFunction::Finish),
                                layer(brick, t - 0.03, LayerFunction::Structure),
                                layer(plaster, 0.015, LayerFunction::Finish),
                            ],
                            "madera" => vec![
                                layer(wood, 0.02, LayerFunction::Finish),
                                layer(brick, t - 0.035, LayerFunction::Structure),
                                layer(plaster, 0.015, LayerFunction::Finish),
                            ],
                            "fence" => vec![
                                layer(render, 0.015, LayerFunction::Finish),
                                layer(brick, t - 0.03, LayerFunction::Structure),
                                layer(render, 0.015, LayerFunction::Finish),
                            ],
                            _ => vec![
                                layer(plaster, 0.015, LayerFunction::Finish),
                                layer(brick, t - 0.03, LayerFunction::Structure),
                                layer(plaster, 0.015, LayerFunction::Finish),
                            ],
                        };
                        let id = m.add_wall_type(format!("{label} {cm} cm"), function, layers)?;
                        wall_types.insert((cm, kind.to_string()), id);
                        id
                    }
                };
                let made = m.add_wall(NewWall {
                    wall_type,
                    level: base,
                    start: Point2::new(num(f[5])?, num(f[6])?),
                    end: Point2::new(num(f[7])?, num(f[8])?),
                    bulge: 0.0,
                    reference: ReferenceLine::Center,
                    height,
                    base_offset: 0.0,
                    top,
                });
                match made {
                    Ok(id) => {
                        report.walls += 1;
                        walls.push(Some(id));
                    }
                    Err(e) => {
                        report.skipped.push(format!("línea {}: muro: {e:?}", n + 1));
                        walls.push(None);
                    }
                }
            }
            "OPEN" => {
                let Some(Some(wall)) = walls.get(num(f[1])? as usize).copied() else {
                    continue;
                };
                let kind = f[2];
                let (w, h, sill, along) = (num(f[3])?, num(f[4])?, num(f[5])?, num(f[6])?);
                let key = (
                    kind.to_string(),
                    (w * 100.0).round() as i64,
                    (h * 100.0).round() as i64,
                );
                let opening_type = match opening_types.get(&key) {
                    Some(id) => *id,
                    None => {
                        use archisim_core::{DoorOperation as D, WindowOperation as W};
                        let (name, op, src) = match kind {
                            "door" => (
                                "Puerta 1 hoja",
                                OpeningKind::Door(D::SingleSwing),
                                &door_type,
                            ),
                            "door2" => (
                                "Puerta 2 hojas",
                                OpeningKind::Door(D::DoubleSwing),
                                &door_type,
                            ),
                            "window" => (
                                "Ventana 2 hojas",
                                OpeningKind::Window(W::DoubleCasement),
                                &window_type,
                            ),
                            "fixed" => (
                                "Acristalamiento fijo",
                                OpeningKind::Window(W::Fixed),
                                &window_type,
                            ),
                            _ => ("Hueco de paso", OpeningKind::Void, &door_type),
                        };
                        let id = m.add_opening_type(NewOpeningType {
                            name: format!("{name} {}×{}", key.1, key.2),
                            kind: op,
                            width: w,
                            height: h,
                            frame_width: src.frame_width,
                            frame_depth: src.frame_depth,
                            frame_material: src.frame_material,
                            panel_material: src.panel_material,
                        })?;
                        opening_types.insert(key, id);
                        id
                    }
                };
                let made = m.add_opening(NewOpening {
                    opening_type,
                    wall,
                    along,
                    sill,
                    opens_to: if f[7] == "ext" {
                        WallSide::Exterior
                    } else {
                        WallSide::Interior
                    },
                    hand: if f[8] == "L" { Hand::Left } else { Hand::Right },
                    setback: None,
                });
                match made {
                    Ok(_) => report.openings += 1,
                    Err(e) => report
                        .skipped
                        .push(format!("línea {}: {kind} de {w:.2} m: {e:?}", n + 1)),
                }
            }
            "CURTAIN" => {
                let (base, top) = (level(&levels, f[1])?, level(&levels, f[2])?);
                let made = m.add_curtain_wall(NewCurtainWall {
                    curtain_type: lib.curtain,
                    level: base,
                    start: Point2::new(num(f[3])?, num(f[4])?),
                    end: Point2::new(num(f[5])?, num(f[6])?),
                    base_offset: 0.0,
                    top: WallTop::Level {
                        level: top,
                        offset: 0.0,
                    },
                    height: 3.0,
                });
                match made {
                    Ok(_) => report.curtains += 1,
                    Err(e) => report
                        .skipped
                        .push(format!("línea {}: muro cortina: {e:?}", n + 1)),
                }
            }
            "SLAB" => {
                let slab_type = slab_type(&mut m, &mut slab_types, f[2], &lib)?;
                let made = m.add_slab(NewSlab {
                    slab_type,
                    level: level(&levels, f[1])?,
                    outline: points(f[3])?,
                    top_offset: 0.0,
                });
                match made {
                    Ok(id) => {
                        report.slabs += 1;
                        last_slab = Some(id);
                    }
                    Err(e) => {
                        last_slab = None;
                        report
                            .skipped
                            .push(format!("línea {}: forjado: {e:?}", n + 1));
                    }
                }
            }
            "HOLE" => {
                if let Some(slab) = last_slab {
                    if let Err(e) = m.add_slab_hole(slab, points(f[1])?) {
                        report
                            .skipped
                            .push(format!("línea {}: hueco: {e:?}", n + 1));
                    }
                }
            }
            "COLUMN" => {
                let (base, top) = (level(&levels, f[1])?, level(&levels, f[2])?);
                let d = num(f[3])?;
                let mm = (d * 1000.0).round() as i64;
                let member_type = match column_types.get(&mm) {
                    Some(id) => *id,
                    None => {
                        let white = material(&mut m, "Lacado blanco", "steel", [240, 240, 236]);
                        let id = m.add_member_type(
                            format!("Pilar circular de acero Ø{}", mm / 10),
                            MemberUsage::Column,
                            white,
                            Profile::Circle { diameter: d },
                        )?;
                        column_types.insert(mm, id);
                        id
                    }
                };
                let made = m.add_column(NewColumn {
                    member_type,
                    level: base,
                    position: Point2::new(num(f[4])?, num(f[5])?),
                    rotation: 0.0,
                    base_offset: 0.0,
                    top: WallTop::Level {
                        level: top,
                        offset: 0.0,
                    },
                    height: 3.0,
                });
                match made {
                    Ok(_) => report.columns += 1,
                    Err(e) => report
                        .skipped
                        .push(format!("línea {}: pilar: {e:?}", n + 1)),
                }
            }
            "LOUVRE" => {
                let lv = level(&levels, f[1])?;
                let (w, h) = (num(f[5])?, num(f[6])?);
                let key = ((w * 100.0).round() as i64, (h * 100.0).round() as i64);
                let object_type = match louvre_types.get(&key) {
                    Some(id) => *id,
                    None => {
                        let family = m.system_family("Panel de lamas orientable")?;
                        let mut values = m.family_defaults(family, ParamScope::Type)?;
                        values.insert("width".into(), Value::Number(w));
                        values.insert("height".into(), Value::Number(h));
                        let id = m.add_object_type(
                            format!("Panel de lamas de madera {}×{}", key.0, key.1),
                            family,
                            values,
                        )?;
                        louvre_types.insert(key, id);
                        id
                    }
                };
                let made = m.add_object(NewObject {
                    level: lv,
                    object_type,
                    position: Point2::new(num(f[2])?, num(f[3])?),
                    rotation: num(f[4])?,
                    base_offset: 0.0,
                });
                match made {
                    Ok(_) => report.louvres += 1,
                    Err(e) => report.skipped.push(format!("línea {}: lama: {e:?}", n + 1)),
                }
            }
            "PLANTER" => {
                let lv = level(&levels, f[1])?;
                let (h, t) = (num(f[2])?, num(f[3])?);
                let centre = points(f[4])?;
                let earth = points(f[5])?;
                // The parapet: a low white wall along each side.
                let cm = (t * 100.0).round() as i64;
                let peto = match wall_types.get(&(cm, "peto".to_string())) {
                    Some(id) => *id,
                    None => {
                        let concrete =
                            material(&mut m, "Hormigón visto blanco", "concrete", [236, 234, 228]);
                        let id = m.add_wall_type(
                            format!("Peto de jardinera {cm} cm"),
                            WallFunction::Exterior,
                            vec![layer(concrete, t, LayerFunction::Structure)],
                        )?;
                        wall_types.insert((cm, "peto".to_string()), id);
                        id
                    }
                };
                for (i, a) in centre.iter().enumerate() {
                    let b = centre[(i + 1) % centre.len()];
                    if (b.x - a.x).hypot(b.y - a.y) < 0.05 {
                        continue;
                    }
                    if let Err(e) = m.add_wall(NewWall {
                        wall_type: peto,
                        level: lv,
                        start: *a,
                        end: b,
                        bulge: 0.0,
                        reference: ReferenceLine::Center,
                        height: h,
                        base_offset: 0.0,
                        top: WallTop::Unconnected,
                    }) {
                        report.skipped.push(format!("línea {}: peto: {e:?}", n + 1));
                    }
                }
                // The earth, its top 5 cm under the parapet's.
                let soil = slab_type(&mut m, &mut slab_types, "tierra", &lib)?;
                if let Err(e) = m.add_slab(NewSlab {
                    slab_type: soil,
                    level: lv,
                    outline: earth.clone(),
                    top_offset: h - 0.05,
                }) {
                    report
                        .skipped
                        .push(format!("línea {}: tierra: {e:?}", n + 1));
                }
                // And the plants, mixed at random over it.
                let mut species = Vec::new();
                for name in f[6].split(';').filter(|s| !s.is_empty()) {
                    match archisim_library::add_object_entry(&mut m, name) {
                        Ok(id) => species.push(id),
                        Err(e) => report
                            .skipped
                            .push(format!("línea {}: especie «{name}»: {e:?}", n + 1)),
                    }
                }
                let count = num(f[7])? as u32;
                if !species.is_empty() {
                    let made = m.add_planting(NewPlanting {
                        name: format!("Jardinera {}", report.planters + 1),
                        level: lv,
                        path: earth,
                        closed: true,
                        smooth: false,
                        arrangement: Arrangement::Area,
                        count,
                        species,
                        size_variation: 0.4,
                        shape_variation: 0.5,
                        spread: 0.7,
                        seed: 17 + report.planters as u64,
                        base_offset: h - 0.05,
                    });
                    match made {
                        Ok(_) => report.plants += count as usize,
                        Err(e) => report
                            .skipped
                            .push(format!("línea {}: plantación: {e:?}", n + 1)),
                    }
                }
                report.planters += 1;
            }
            "ROOM" => {
                let label = points(f[4])?.first().copied().unwrap_or_default();
                let made = m.add_room(NewRoom {
                    level: level(&levels, f[1])?,
                    number: f[2].to_string(),
                    name: f[3].to_string(),
                    outline: points(f[5])?,
                    holes: f
                        .get(7)
                        .map(|h| {
                            h.split(';')
                                .filter(|s| !s.trim().is_empty())
                                .map(points)
                                .collect::<R<Vec<_>>>()
                        })
                        .transpose()?
                        .unwrap_or_default(),
                    label,
                    bounded: false,
                });
                match made {
                    Ok(_) => report.rooms += 1,
                    Err(e) => {
                        report
                            .skipped
                            .push(format!("línea {}: local «{}»: {e:?}", n + 1, f[3]))
                    }
                }
            }
            "" => {}
            other => report
                .skipped
                .push(format!("línea {}: no sé qué es «{other}»", n + 1)),
        }
    }
    Ok((m, report))
}

/// Reads the description, builds the project and saves it.
pub fn run(spec: &Path, out: &Path, name: &str) -> R<()> {
    let text = std::fs::read_to_string(spec)?;
    let (m, r) = build(name, &text)?;
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }
    archisim_file::save(&m, out)?;
    println!(
        "  {} muros, {} puertas y ventanas, {} muros cortina, {} forjados, {} locales",
        r.walls, r.openings, r.curtains, r.slabs, r.rooms
    );
    if r.columns + r.louvres + r.planters > 0 {
        println!(
            "  {} pilares, {} paneles de lamas, {} jardineras con {} plantas",
            r.columns, r.louvres, r.planters, r.plants
        );
    }
    if !r.skipped.is_empty() {
        println!("  Sin colocar ({}):", r.skipped.len());
        for s in &r.skipped {
            println!("    {s}");
        }
    }
    println!("  Proyecto: {}", out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_small_description_builds() {
        let text = "LEVEL|Planta baja|0\nLEVEL|Cubierta|3\n\
WALL|Planta baja|Cubierta|0.25|ext|0|0|6|0\n\
OPEN|0|door|0.9|2.1|0|3|int|L\n\
SLAB|Planta baja|solera|0,0 6,0 6,4 0,4\n\
ROOM|Planta baja|PB.01|Sala|3,2|0,0 6,0 6,4 0,4|24\n";
        let (m, r) = super::build("prueba", text).unwrap();
        assert!(r.skipped.is_empty(), "{:?}", r.skipped);
        assert_eq!((r.walls, r.openings, r.slabs, r.rooms), (1, 1, 1, 1));
        assert_eq!(m.walls().count(), 1);
    }

    #[test]
    fn columns_louvres_and_planters_build() {
        let text = "LEVEL|Planta primera|0\nLEVEL|Cubierta|4\n\
WALL|Planta primera|Cubierta|0.30|blanco|0|0|8|0\n\
SLAB|Planta primera|volado|-1,-2 9,-2 9,4 -1,4\n\
SLAB|Cubierta|grava|-1,-2 9,-2 9,4 -1,4\n\
COLUMN|Planta primera|Cubierta|0.20|1|-1.5\n\
LOUVRE|Planta primera|3|-1.2|0.6|0.80|3.50\n\
LOUVRE|Planta primera|4|-1.2|-0.3|0.80|3.50\n\
PLANTER|Cubierta|0.60|0.12|0.06,0.06 5.94,0.06 5.94,0.94 0.06,0.94|0.13,0.13 5.87,0.13 5.87,0.87 0.13,0.87|Palmito;Romero|6\n";
        let (m, r) = super::build("prueba", text).unwrap();
        assert!(r.skipped.is_empty(), "{:?}", r.skipped);
        assert_eq!((r.columns, r.louvres, r.planters, r.plants), (1, 2, 1, 6));
        // The two louvre panels share their type, and are turned as drawn.
        assert_eq!(
            m.object_types()
                .filter(|t| t.name == "Panel de lamas de madera 80×350")
                .count(),
            1
        );
        // Four parapets and the façade; the earth is a slab too.
        assert_eq!(m.walls().count(), 5);
        assert_eq!(m.slabs().count(), 3);
        // The façade is rendered white outside (its first layer).
        let w = m.walls().next().unwrap();
        let t = m.wall_type(w.wall_type).unwrap();
        assert_eq!(m.material(t.layers[0].material).unwrap().name, "Cal blanca");
    }
}
